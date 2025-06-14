use alloc::{
    boxed::Box,
    collections::BTreeMap,
    sync::{Arc, Weak},
};
use libkernel::error::{KernelError, Result};
use log::{debug, info, warn};

use crate::{
    drivers::Driver,
    sync::{OnceLock, SpinLock},
};

pub mod cpu_messenger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    EdgeRising,
    EdgeFalling,
    LevelHigh,
    LevelLow,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum InterruptDescriptor {
    Spi(usize),
    Ppi(usize),
    Ipi(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptConfig {
    pub descriptor: InterruptDescriptor,
    pub trigger: TriggerMode,
}

/// Represents an active interrupt being handled. Implementors should signal
/// end-of-interrupt on drop.
pub trait InterruptContext: Send {
    /// The interrupt descriptor.
    fn descriptor(&self) -> InterruptDescriptor;
}

pub trait InterruptController: Send + Sync {
    fn enable_interrupt(&mut self, i: InterruptConfig);

    fn disable_interrupt(&mut self, i: InterruptDescriptor);

    /// Returns an active interrupt, wrapped in a context that
    /// will automatically signal end-of-interrupt when dropped.
    fn read_active_interrupt(&mut self) -> Option<Box<dyn InterruptContext>>;

    /// Sends an IPI to the given CPU ID.
    fn raise_ipi(&mut self, target_cpu_id: usize);

    /// Enable the interrupt controller for this core. This is the entry point
    /// for secondaries only, the primary CPU should have initalized via the
    /// creation of the interupt controller object.
    fn enable_core(&mut self, cpu_id: usize);

    fn parse_fdt_interrupt_regs(
        &self,
        iter: &mut dyn Iterator<Item = u32>,
    ) -> Result<InterruptConfig>;
}

pub trait InterruptHandler: Send + Sync {
    fn handle_irq(&self, desc: InterruptDescriptor);
}

struct InterruptManagerInner {
    claimed_interrupts: BTreeMap<InterruptDescriptor, ClaimedInterrupt>,
    controller: Arc<SpinLock<dyn InterruptController>>,
}

pub struct InterruptManager {
    name: &'static str,
    inner: SpinLock<InterruptManagerInner>,
}

impl InterruptManager {
    pub fn new(name: &'static str, driver: Arc<SpinLock<dyn InterruptController>>) -> Arc<Self> {
        // Always enable IPI 0 for kernel use.
        driver.lock_save_irq().enable_interrupt(InterruptConfig {
            descriptor: InterruptDescriptor::Ipi(0),
            trigger: TriggerMode::EdgeFalling,
        });

        Arc::new(Self {
            name,
            inner: SpinLock::new(InterruptManagerInner {
                claimed_interrupts: BTreeMap::new(),
                controller: driver,
            }),
        })
    }

    pub fn parse_fdt_interrupt_regs(
        &self,
        iter: &mut dyn Iterator<Item = u32>,
    ) -> Result<InterruptConfig> {
        self.inner
            .lock_save_irq()
            .controller
            .lock_save_irq()
            .parse_fdt_interrupt_regs(iter)
    }

    pub fn claim_interrupt<T, FConstructor>(
        self: &Arc<Self>,
        config: InterruptConfig,
        constructor: FConstructor,
    ) -> Result<Arc<T>>
    where
        T: 'static + Send + Sync + Driver + InterruptHandler,
        FConstructor: FnOnce(ClaimedInterrupt) -> T,
    {
        let mut inner = self.inner.lock_save_irq();

        if inner.claimed_interrupts.contains_key(&config.descriptor) {
            return Err(KernelError::InUse);
        }

        let driver: Arc<T> = Arc::new_cyclic(|driver_weak: &Weak<T>| {
            let handle = ClaimedInterrupt {
                desc: config.descriptor,
                manager: Arc::clone(self),
                handler: driver_weak.clone(),
            };

            let driver = constructor(handle.clone());

            inner.claimed_interrupts.insert(config.descriptor, handle);

            driver
        });

        inner.controller.lock_save_irq().enable_interrupt(config);

        debug!(
            "Device {} claimed interrupt: {:?}",
            driver.name(),
            config.descriptor,
        );

        Ok(driver)
    }

    fn remove_interrupt(&self, desc: InterruptDescriptor) {
        let mut inner = self.inner.lock_save_irq();
        inner.claimed_interrupts.remove(&desc);
        inner.controller.lock_save_irq().disable_interrupt(desc);
    }

    pub fn handle_interrupt(&self) {
        let mut inner = self.inner.lock_save_irq();
        let ctx = inner.controller.lock_save_irq().read_active_interrupt();

        if let Some(ctx) = ctx {
            let desc = ctx.descriptor();
            if let Some(irq_handle) = inner.claimed_interrupts.get_mut(&desc) {
                match irq_handle.handler.upgrade() {
                    Some(x) => x.handle_irq(ctx.descriptor()),
                    None => warn!("IRQ fired for stale IRQ handle"),
                }
            }
        }
    }

    pub fn raise_ipi(&self, cpu: usize) {
        let inner = self.inner.lock_save_irq();
        inner.controller.lock_save_irq().raise_ipi(cpu);
    }

    pub fn enable_core(&self, cpu_id: usize) {
        self.inner
            .lock_save_irq()
            .controller
            .lock_save_irq()
            .enable_core(cpu_id);
    }
}

impl Driver for InterruptManager {
    fn name(&self) -> &'static str {
        self.name
    }

    fn as_interrupt_manager(self: Arc<Self>) -> Option<Arc<Self>> {
        Arc::downcast(self).ok()
    }
}

#[derive(Clone)]
pub struct ClaimedInterrupt {
    desc: InterruptDescriptor,
    manager: Arc<InterruptManager>,
    handler: Weak<dyn InterruptHandler>,
}

impl Drop for ClaimedInterrupt {
    fn drop(&mut self) {
        self.manager.remove_interrupt(self.desc);
    }
}

static ROOT_INTERRUPT_CONTROLLER: OnceLock<Arc<InterruptManager>> = OnceLock::new();

pub fn set_interrupt_root(root: Arc<InterruptManager>) {
    info!("Setting device {} as the interrupt root", root.name());
    if ROOT_INTERRUPT_CONTROLLER.set(root).is_err() {
        panic!("Should only have one interrupt root");
    }
}

pub fn get_interrupt_root() -> Option<Arc<InterruptManager>> {
    ROOT_INTERRUPT_CONTROLLER.get().cloned()
}

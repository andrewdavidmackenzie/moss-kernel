use core::arch::asm;
use core::fmt;

/// Instruction Fault Status Code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ifsc(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfscCategory {
    TranslationFault,
    PermissionFault,
    AccessFlagFault,
    Other,
}

impl Ifsc {
    pub fn category(self) -> IfscCategory {
        match self.0 {
            // Translation faults (PTE not present)
            0b000100..=0b000111 | // levels 0-3
            0b101010 | 0b101011 | // level -2, -1
            0b000000..=0b000011 | // address size faults level 0–3
            0b101001 | 0b101100 => // address size faults -1, -2
                IfscCategory::TranslationFault,

            // Permission faults
            0b001100..=0b001111 => IfscCategory::PermissionFault,

            // Access flag faults (PTE present but access tracking not enabled)
            0b001000..=0b001011 => IfscCategory::AccessFlagFault,

            // Everything else
            _ => IfscCategory::Other,
        }
    }
}

impl From<u8> for Ifsc {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

/// Instruction Specific Syndrome for Data/Instruction Aborts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbortIss {
    pub ifsc: Ifsc,
    pub write: bool,
    pub far: Option<u64>,
}

impl From<u64> for AbortIss {
    fn from(iss: u64) -> Self {
        let far = if (iss >> 10) & 1 == 0 {
            let far_val;
            unsafe {
                asm!("mrs {}, far_el1",
                     out(reg) far_val,
                     options(nomem, nostack, preserves_flags));
            }
            Some(far_val)
        } else {
            None
        };

        Self {
            ifsc: ((iss & 0b11_1111) as u8).into(),
            write: (iss >> 6) & 1 == 1,
            far,
        }
    }
}

/// Instruction Specific Syndrome for SVC/HVC/SMC exceptions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SvcIss {
    pub imm: u16,
}

impl From<u64> for SvcIss {
    fn from(iss: u64) -> Self {
        Self {
            imm: (iss & 0xFFFF) as u16,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exception {
    InstrAbortLowerEL(AbortIss),
    InstrAbortCurrentEL(AbortIss),
    DataAbortLowerEL(AbortIss),
    DataAbortCurrentEL(AbortIss),
    SVC64(SvcIss),
    SVC32(SvcIss),

    TrappedWFIorWFE(u64),
    TrappedMCRorMRC(u64),   // A32
    TrappedMCRRorMRRC(u64), // A32
    TrappedMCRorMRC2(u64),  // A32
    TrappedLDCorSTC(u64),   // A32
    TrappedFP(u64),
    TrappedMRRC(u64), // A32
    BranchTarget(u64),
    IllegalExecutionState(u64),
    HVC64(u64),
    SMC64(u64),
    TrappedMsrMrs(u64),
    TrappedSve(u64),
    PointerAuth(u64),
    PCAlignmentFault(u64),
    SPAlignmentFault(u64),
    TrappedFP32(u64), // A32
    TrappedFP64(u64),
    SError(u64),
    BreakpointLowerEL(u64),
    BreakpointCurrentEL(u64),
    SoftwareStepLowerEL(u64),
    SoftwareStepCurrentEL(u64),
    WatchpointLowerEL(u64),
    WatchpointCurrentEL(u64),
    Bkpt32(u64),
    Brk64(u64),

    // Fallback for unknown exception classes
    Unknown(u64),
}

impl From<Esr> for Exception {
    fn from(esr: Esr) -> Self {
        let value = esr.raw();
        let ec = (value >> 26) & 0b11_1111; // Exception Class bits
        let iss = value & 0x1FFFFFF; // Instruction Specific Syndrome bits

        match ec {
            0b10_0000 => Exception::InstrAbortLowerEL(AbortIss::from(iss)),
            0b10_0001 => Exception::InstrAbortCurrentEL(AbortIss::from(iss)),
            0b10_0100 => Exception::DataAbortLowerEL(AbortIss::from(iss)),
            0b10_0101 => Exception::DataAbortCurrentEL(AbortIss::from(iss)),
            0b01_0101 => Exception::SVC64(SvcIss::from(iss)),
            0b01_0001 => Exception::SVC32(SvcIss::from(iss)),

            // Map the rest to their raw ISS variants
            0b00_0001 => Exception::TrappedWFIorWFE(iss),
            0b00_0011 => Exception::TrappedMCRorMRC(iss),
            0b00_0100 => Exception::TrappedMCRRorMRRC(iss),
            0b00_0101 => Exception::TrappedMCRorMRC2(iss),
            0b00_0110 => Exception::TrappedLDCorSTC(iss),
            0b00_0111 => Exception::TrappedFP(iss),
            0b00_1100 => Exception::TrappedMRRC(iss),
            0b00_1101 => Exception::BranchTarget(iss),
            0b00_1110 => Exception::IllegalExecutionState(iss),
            0b01_0110 => Exception::HVC64(iss),
            0b01_0111 => Exception::SMC64(iss),
            0b01_1000 => Exception::TrappedMsrMrs(iss),
            0b01_1001 => Exception::TrappedSve(iss),
            0b01_1100 => Exception::PointerAuth(iss),
            0b10_0010 => Exception::PCAlignmentFault(iss),
            0b10_0110 => Exception::SPAlignmentFault(iss),
            0b10_1000 => Exception::TrappedFP32(iss),
            0b10_1100 => Exception::TrappedFP64(iss),
            0b10_1111 => Exception::SError(iss),
            0b11_0000 => Exception::BreakpointLowerEL(iss),
            0b11_0001 => Exception::BreakpointCurrentEL(iss),
            0b11_0010 => Exception::SoftwareStepLowerEL(iss),
            0b11_0011 => Exception::SoftwareStepCurrentEL(iss),
            0b11_0100 => Exception::WatchpointLowerEL(iss),
            0b11_0101 => Exception::WatchpointCurrentEL(iss),
            0b11_1000 => Exception::Bkpt32(iss),
            0b11_1100 => Exception::Brk64(iss),

            unknown_ec => Exception::Unknown(unknown_ec),
        }
    }
}

/// Represents the raw Exception Syndrome Register (ESR_EL1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Esr {
    value: u64,
}

impl Esr {
    /// Read the current ESR_EL1 value.
    pub fn read_el1() -> Self {
        let value;
        unsafe {
            asm!("mrs {}, esr_el1",
                 out(reg) value,
                 options(nomem, nostack, preserves_flags));
        }
        Self { value }
    }

    /// Get the raw, undecoded ESR value.
    pub fn raw(&self) -> u64 {
        self.value
    }

    /// Decode the raw ESR value into a structured Exception.
    pub fn decode(self) -> Exception {
        Exception::from(self)
    }
}

impl fmt::Debug for Esr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Esr")
            .field("value", &format_args!("{:#x}", self.value))
            .field("decoded", &self.decode())
            .finish()
    }
}

use crate::igvm::{BootPageAcceptance, StartupMemoryType, HV_PAGE_SIZE};
use igvm_defs::IgvmVariableHeaderType;
use igvm_parser::hv_defs::Vtl;
use igvm_parser::registers::X86Register;
use range_map_vec::{Entry, RangeMap};
use vm_memory::GuestMemoryMmap;

use std::collections::HashMap;
use std::mem::Discriminant;
use thiserror::Error;
use vm_memory::bitmap::AtomicBitmap;
use vm_memory::{Bytes, GuestAddress, GuestAddressSpace, GuestMemoryAtomic};
use vm_memory::{GuestMemory, GuestMemoryRegion};

#[derive(Debug)]
pub struct ImportRegion {
    pub page_base: u64,
    pub page_count: u64,
    pub acceptance: BootPageAcceptance,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("overlaps with existing import region {0:?}")]
    OverlapsExistingRegion(ImportRegion),
    #[error("invalid parameter area index {0:?}")]
    InvalidParameterAreaIndex(ParameterAreaIndex),
    #[error("invalid vtl")]
    InvalidVtl,
    #[error("memory unvailable")]
    MemoryUnavailable,
    #[error("invalid vp context memory")]
    InvalidVpContextMemory(&'static str),
    #[error("data larger than imported region")]
    DataTooLarge,
    #[error("no vp context page set")]
    NoVpContextPageSet,
    #[error("overlaps existing relocation region")]
    RelocationOverlap,
    #[error("region alignment is not aligned to 4K")]
    RelocationAlignment,
    #[error("relocation base gpa is not aligned to relocation alignment")]
    RelocationBaseGpa,
    #[error("relocation minimum gpa is not aligned to relocation alignment")]
    RelocationMinimumGpa,
    #[error("relocation maximum gpa is not aligned to relocation alignment")]
    RelocationMaximumGpa,
    #[error("relocation size is not 4K aligned")]
    RelocationSize,
    #[error("page table relocation is already set, only a single allowed")]
    PageTableRelocationSet,
    #[error("page table relocation used size is greater than the region size")]
    PageTableUsedSize,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ParameterAreaIndex(pub u32);

#[derive(Debug)]
pub struct Loader {
    memory: GuestMemoryAtomic<GuestMemoryMmap<AtomicBitmap>>,
    regs: HashMap<Discriminant<X86Register>, X86Register>,
    accepted_ranges: RangeMap<u64, BootPageAcceptance>,
    max_vtl: Vtl,
    bytes_written: u64,
}

impl Loader {
    pub fn new(memory: GuestMemoryAtomic<GuestMemoryMmap<AtomicBitmap>>, max_vtl: Vtl) -> Loader {
        Loader {
            memory,
            regs: HashMap::new(),
            accepted_ranges: RangeMap::new(),
            max_vtl,
            bytes_written: 0,
        }
    }

    pub fn get_initial_regs(self) -> Vec<X86Register> {
        self.regs.into_values().collect()
    }
    /// Accept a new page range with a given acceptance into the map of accepted ranges.
    pub fn accept_new_range(
        &mut self,
        page_base: u64,
        page_count: u64,
        acceptance: BootPageAcceptance,
    ) -> Result<(), Error> {
        let page_end = page_base + page_count - 1;
        match self.accepted_ranges.entry(page_base..=page_end) {
            Entry::Overlapping(entry) => {
                let &(overlap_start, overlap_end, overlap_acceptance) = entry.get();

                Err(Error::OverlapsExistingRegion(ImportRegion {
                    page_base: overlap_start,
                    page_count: overlap_end - overlap_start + 1,
                    acceptance: overlap_acceptance,
                }))
            }
            Entry::Vacant(entry) => {
                entry.insert(acceptance);
                Ok(())
            }
        }
    }
    pub fn gets_total_bytes_written(self) -> u64 {
        self.bytes_written
    }

    pub fn import_pages(
        &mut self,
        page_base: u64,
        page_count: u64,
        acceptance: BootPageAcceptance,
        data: &[u8],
    ) -> Result<(), Error> {
        // Track accepted ranges for duplicate imports.
        self.accept_new_range(page_base, page_count, acceptance)?;

        // Page count must be larger or equal to data.
        if page_count * HV_PAGE_SIZE < data.len() as u64 {
            return Err(Error::DataTooLarge);
        }

        self.memory
            .memory()
            .write(data, GuestAddress(page_base * HV_PAGE_SIZE))
            .map_err(|e| {
                debug!("Importing pages failed due to MemoryError");
                Error::MemoryUnavailable
            })?;
        self.bytes_written += page_count * HV_PAGE_SIZE;
        Ok(())
    }

    pub fn import_vp_register(&mut self, vtl: Vtl, register: X86Register) -> Result<(), Error> {
        // Only importing to the max VTL for registers is currently allowed, as only one set of registers is stored.
        if vtl != self.max_vtl {
            return Err(Error::InvalidVtl);
        }

        let entry = self.regs.entry(std::mem::discriminant(&register));
        match entry {
            std::collections::hash_map::Entry::Occupied(_) => {
                panic!("duplicate register import {:?}", register)
            }
            std::collections::hash_map::Entry::Vacant(ve) => ve.insert(register),
        };

        Ok(())
    }

    pub fn verify_startup_memory_available(
        &mut self,
        page_base: u64,
        page_count: u64,
        memory_type: StartupMemoryType,
    ) -> Result<(), Error> {
        // Allow Vtl2ProtectableRam only if VTL2 is enabled.
        if self.max_vtl == Vtl::Vtl2 {
            match memory_type {
                StartupMemoryType::Ram => {}
                StartupMemoryType::Vtl2ProtectableRam => {
                    // TODO: Should enable VTl2 memory protections on this region? Or do we allow VTL2 memory protections
                    //       on the whole address space when VTL memory protections work?
                    warn!(
                        "vtl2 protectable ram requested: {:?} {:?}",
                        page_base, page_count,
                    );
                }
            }
        } else if memory_type != StartupMemoryType::Ram {
            return Err(Error::MemoryUnavailable);
        }

        let mut memory_found = false;

        for range in self.memory.memory().iter() {
            // Today, the memory layout only describes normal ram and mmio. Thus the memory
            // request must live completely within a single range, since any gaps are mmio.
            let base_address = page_base * HV_PAGE_SIZE;
            let end_address = base_address + (page_count * HV_PAGE_SIZE) - 1;

            if base_address >= range.start_addr().0 && base_address < range.last_addr().0 {
                if end_address > range.last_addr().0 {
                    debug!("startup memory end bigger than the current range");
                    return Err(Error::MemoryUnavailable);
                }

                memory_found = true;
            }
        }

        if memory_found {
            Ok(())
        } else {
            debug!("no valid memory range available for startup memory verify");
            Err(Error::MemoryUnavailable)
        }
    }
}

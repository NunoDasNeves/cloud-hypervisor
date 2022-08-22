// Copyright 2020 Arm Limited (or its affiliates). All rights reserved.
// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//
// Memory layout of AArch64 guest:
//
// Physical  +---------------------------------------------------------------+
// address   |                                                               |
// end       |                                                               |
//           ~                   ~                       ~                   ~
//           |                                                               |
//           |                      Highmem PCI MMIO space                   |
//           |                                                               |
// RAM end   +---------------------------------------------------------------+
// (dynamic, |                                                               |
// including |                                                               |
// hotplug   ~                   ~                       ~                   ~
// memory)   |                                                               |
//           |                            DRAM                               |
//           |                                                               |
//           |                                                               |
// 4GB       +---------------------------------------------------------------+
//           |                      32-bit devices hole                      |
// 4GB-64M   +---------------------------------------------------------------+
//           |                                                               |
//           |                                                               |
//           |                            DRAM                               |
//           |                                                               |
//           |                                                               |
// 1GB       +---------------------------------------------------------------+
//           |                                                               |
//           |                        PCI MMCONFIG space                     |
//           |                                                               |
// 768 M     +---------------------------------------------------------------+
//           |                                                               |
//           |                                                               |
//           |                           PCI MMIO space                      |
//           |                                                               |
// 256 M     +---------------------------------------------------------------|
//           |                                                               |
//           |                        Legacy devices space                   |
//           |                                                               |
// 144 M     +---------------------------------------------------------------|
//           |                                                               |
//           |                    Reserved (now GIC is here)                 |
//           |                                                               |
//  4  M     +---------------------------------------------------------------+
//           |                          UEFI flash                           |
// 0GB       +---------------------------------------------------------------+
//
//

use vm_memory::GuestAddress;

/// 0x0 ~ 0x40_0000 (4 MiB) is reserved to UEFI
/// UEFI binary size is required less than 3 MiB, reserving 4 MiB is enough.
pub const UEFI_START: GuestAddress = GuestAddress(0);
pub const UEFI_SIZE: u64 = 0x040_0000;

/// Below this address will reside the GIC, above this address will reside the MMIO devices.
const MAPPED_IO_START: GuestAddress = GuestAddress(0x0900_0000);

/// See kernel file arch/arm64/include/uapi/asm/kvm.h for the GIC related definitions.
/// 0x08ff_0000 ~ 0x0900_0000 is reserved for GICv3 Distributor
pub const GIC_V3_DIST_SIZE: u64 = 0x01_0000;
pub const GIC_V3_DIST_START: GuestAddress = GuestAddress(MAPPED_IO_START.0 - GIC_V3_DIST_SIZE);
/// Below 0x08ff_0000 is reserved for GICv3 Redistributor.
/// The size defined here is for each vcpu.
/// The total size is 'number_of_vcpu * GIC_V3_REDIST_SIZE'
pub const GIC_V3_REDIST_SIZE: u64 = 0x02_0000;
/// Below Redistributor area is GICv3 ITS
pub const GIC_V3_ITS_SIZE: u64 = 0x02_0000;

/// Space 0x0900_0000 ~ 0x0905_0000 is reserved for legacy devices.
pub const LEGACY_SERIAL_MAPPED_IO_START: GuestAddress = MAPPED_IO_START;
pub const LEGACY_RTC_MAPPED_IO_START: GuestAddress = GuestAddress(0x0901_0000);
pub const LEGACY_GPIO_MAPPED_IO_START: GuestAddress = GuestAddress(0x0902_0000);

/// Space 0x0905_0000 ~ 0x0906_0000 is reserved for pcie io address
pub const MEM_PCI_IO_START: GuestAddress = GuestAddress(0x0905_0000);
pub const MEM_PCI_IO_SIZE: u64 = 0x10000;

/// Starting from 0x1000_0000 (256MiB) to 0x3000_0000 (768MiB) is used for PCIE MMIO
pub const MEM_32BIT_DEVICES_START: GuestAddress = GuestAddress(0x1000_0000);
pub const MEM_32BIT_DEVICES_SIZE: u64 = 0x2000_0000;

/// PCI MMCONFIG space (start: after the device space at 1 GiB, length: 256MiB)
pub const PCI_MMCONFIG_START: GuestAddress = GuestAddress(0x3000_0000);
pub const PCI_MMCONFIG_SIZE: u64 = 256 << 20;
// One bus with potentially 256 devices (32 slots x 8 functions).
pub const PCI_MMIO_CONFIG_SIZE_PER_SEGMENT: u64 = 4096 * 256;

/// Start of RAM.
pub const RAM_START: GuestAddress = GuestAddress(0x4000_0000);

/// 32-bit reserved area: 64MiB before 4GiB
pub const MEM_32BIT_RESERVED_START: GuestAddress = GuestAddress(0xfc00_0000);
pub const MEM_32BIT_RESERVED_SIZE: u64 = 0x0400_0000;

/// Start of 64-bit RAM.
pub const RAM_64BIT_START: GuestAddress = GuestAddress(0x1_0000_0000);

/// Kernel command line maximum size.
/// As per `arch/arm64/include/uapi/asm/setup.h`.
pub const CMDLINE_MAX_SIZE: usize = 2048;

/// FDT is at the beginning of RAM.
/// Maximum size of the device tree blob as specified in https://www.kernel.org/doc/Documentation/arm64/booting.txt.
pub const FDT_START: GuestAddress = RAM_START;
pub const FDT_MAX_SIZE: u64 = 0x20_0000;

/// Put ACPI table above dtb
pub const ACPI_START: GuestAddress = GuestAddress(RAM_START.0 + FDT_MAX_SIZE);
pub const ACPI_MAX_SIZE: u64 = 0x20_0000;
pub const RSDP_POINTER: GuestAddress = ACPI_START;

/// Kernel start after FDT and ACPI
pub const KERNEL_START: GuestAddress = GuestAddress(ACPI_START.0 + ACPI_MAX_SIZE);

/// Pci high memory base
pub const PCI_HIGH_BASE: GuestAddress = GuestAddress(0x2_0000_0000);

// As per virt/kvm/arm/vgic/vgic-kvm-device.c we need
// the number of interrupts our GIC will support to be:
// * bigger than 32
// * less than 1023 and
// * a multiple of 32.
// We are setting up our interrupt controller to support a maximum of 256 interrupts.
/// First usable interrupt on aarch64
pub const IRQ_BASE: u32 = 32;

/// Number of supported interrupts
pub const IRQ_NUM: u32 = 256;


use versionize::{VersionMap, Versionize, VersionizeResult, VersionizeError};
use versionize_derive::Versionize;
use serde::{Serialize, Deserialize};

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize, Versionize)]
pub enum RegionName {
    UEFI,
    GIC_V3_ITS,
    GIC_V3_REDIST,
    GIC_V3_DIST,
    LEGACY_SERIAL_MAPPED_IO,
    LEGACY_RTC_MAPPED_IO,
    LEGACY_GPIO_MAPPED_IO,
    MEM_PCI_IO,
    MEM_32BIT_DEVICES,
    PCI_MMCONFIG,
    RAM,
    MEM_32BIT_RESERVED,
    RAM_64BIT,
    PLATFORM_DEVICES,
}

use std::collections::BTreeMap;
use vm_memory::Address;
use vm_memory::GuestUsize;
use crate::ArchMemRegion;

pub fn arch_memory_regions_from_vec(
    v: &Vec<ArchMemRegion>
) -> BTreeMap<RegionName, (GuestAddress, GuestUsize)> {
    let mut regions = BTreeMap::<RegionName, (GuestAddress, GuestUsize)>::new();
    for region in v.iter() {
        regions.insert(region.name, (GuestAddress(region.base), region.size));
    }
    regions
}

pub fn arch_memory_regions(
    ram_size: GuestUsize,
    vcpus: u64,
) -> BTreeMap<RegionName, (GuestAddress, GuestUsize)> {
    use RegionName::*;
    let mut regions = BTreeMap::<RegionName, (GuestAddress, GuestUsize)>::new();

    regions.insert(UEFI, (UEFI_START, UEFI_SIZE));

    // GIC
    let gic_redists_size = vcpus * GIC_V3_REDIST_SIZE;
    let gic_redist_start = GuestAddress(GIC_V3_DIST_START.0 - gic_redists_size);
    let gic_its_start = GuestAddress(gic_redist_start.0 - GIC_V3_ITS_SIZE);

    regions.insert(GIC_V3_DIST, (GIC_V3_DIST_START, GIC_V3_DIST_SIZE));
    regions.insert(GIC_V3_REDIST, (gic_redist_start, gic_redists_size));
    regions.insert(GIC_V3_ITS, (gic_its_start, GIC_V3_ITS_SIZE));

    // Legacy MMIO
    regions.insert(
        LEGACY_SERIAL_MAPPED_IO,
        (LEGACY_SERIAL_MAPPED_IO_START, 0x1000),
    );
    regions.insert(LEGACY_RTC_MAPPED_IO, (LEGACY_RTC_MAPPED_IO_START, 0x1000));
    regions.insert(LEGACY_GPIO_MAPPED_IO, (LEGACY_GPIO_MAPPED_IO_START, 0x1000));

    // PCI
    regions.insert(MEM_PCI_IO, (MEM_PCI_IO_START, MEM_PCI_IO_SIZE));
    regions.insert(
        MEM_32BIT_DEVICES,
        (MEM_32BIT_DEVICES_START, MEM_32BIT_DEVICES_SIZE),
    );
    regions.insert(PCI_MMCONFIG, (PCI_MMCONFIG_START, PCI_MMCONFIG_SIZE));

    // RAM space
    let ram_32bit_space_size = MEM_32BIT_RESERVED_START.unchecked_offset_from(RAM_START);

    // Case1: guest memory fits before the gap
    if ram_size <= ram_32bit_space_size {
        regions.insert(RAM, (RAM_START, ram_size));
    // Case2: guest memory extends beyond the gap
    } else {
        // Push memory before the gap
        regions.insert(RAM, (RAM_START, ram_32bit_space_size));
        // Other memory is placed after 4GiB
        regions.insert(
            RAM_64BIT,
            (RAM_64BIT_START, (ram_size - ram_32bit_space_size)),
        );
    }

    regions.insert(
        PCI_MMCONFIG,
        (MEM_32BIT_RESERVED_START, MEM_32BIT_RESERVED_SIZE),
    );

    // MemoryManager will add the PLATFORM_DEVICES region

    regions
}

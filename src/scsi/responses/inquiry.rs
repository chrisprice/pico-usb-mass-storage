use overlay_macro::overlay;

use crate::scsi::enums::{
    VersionDescriptor,
    // TargetPortGroupSupport,
    // SpcVersion,
    // PeripheralQualifier,
    // PeripheralDeviceType,
    // ResponseDataFormat,
};

// ASCII space is used to pad shorter string identifiers as per SPC
const ASCII_SPACE: u8 = 0x20;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct InquiryResponse {
    #[bit_byte(7, 5, 0, 0)]
    peripheral_qualifier: u8, //PeripheralQualifier,

    #[bit_byte(4, 0, 0, 0)]
    peripheral_device_type: u8, //PeripheralDeviceType,

    ///A removable medium ( RMB ) bit set to zero indicates that the medium is not removable. A RMB bit set to one indicates that the medium is removable.
    #[bit_byte(7, 7, 1, 1)]
    removable_medium: bool,

    ///The VERSION field indicates the implemented version of this standard and is defined in table 142
    #[bit_byte(7, 0, 2, 2)]
    pub version: u8, //SpcVersion,

    ///The Normal ACA Supported (NORMACA) bit set to one indicates that the device server supports a NACA bit set to one in the CDB CONTROL byte and supports the ACA task attribute (see SAM-4). A N ORM ACA bit set to zero indicates that the device server does not support a NACA bit set to one and does not support the ACA task attribute.
    #[bit_byte(5, 5, 3, 3)]
    normal_aca: bool,

    ///A hierarchical support (HISUP) bit set to zero indicates the SCSI target device does not use the hierarchical addressing model to assign LUNs to logical units. A H I S UP bit set to one indicates the SCSI target device uses the hierarchical addressing model to assign LUNs to logical units.
    #[bit_byte(4, 4, 3, 3)]
    hierarchical_support: bool,

    ///The RESPONSE DATA FORMAT field indicates the format of the standard INQUIRY data and shall be set as shown in table 139. A RESPONSE DATA FORMAT field set to 2h indicates that the standard INQUIRY data is in the format defined in this standard. Response data format values less than 2h are obsolete. Response data format values greater than 2h are reserved.
    #[bit_byte(3, 0, 3, 3)]
    response_data_format: u8, //ResponseDataFormat,

    ///The ADDITIONAL LENGTH field indicates the length in bytes of the remaining standard INQUIRY data. The relationship between the ADDITIONAL LENGTH field and the CDB ALLOCATION LENGTH field is defined in 4.3.5.6.
    ///Set to total length in bytes minus 4
    #[bit_byte(7, 0, 4, 4)]
    pub additional_length: u8,

    ///An SCC Supported ( SCCS ) bit set to one indicates that the SCSI target device contains an embedded storage array controller component that is addressable through this logical unit. See SCC-2 for details about storage array controller devices. An SCCS bit set to zero indicates that no embedded storage array controller component is addressable through this logical unit.
    #[bit_byte(7, 7, 5, 5)]
    scc_supported: bool,

    ///An Access Controls Coordinator ( ACC ) bit set to one indicates that the SCSI target device contains an access controls coordinator (see 3.1.4) that is addressable through this logical unit. An ACC bit set to zero indicates that no access controls coordinator is addressable through this logical unit. If the SCSI target device contains an access controls coordinator that is addressable through any logical unit other than the ACCESS CONTROLS well known logical unit (see 8.3), then the ACC bit shall be set to one for LUN 0.
    #[bit_byte(6, 6, 5, 5)]
    access_controls_coordinator: bool,

    ///The contents of the target port group support ( TPGS ) field (see table 143) indicate the support for asymmetric logical unit access (see 5.11).
    #[bit_byte(5, 4, 5, 5)]
    target_port_group_support: u8, //TargetPortGroupSupport,

    ///A Third-Party Copy (3PC) bit set to one indicates that the SCSI target device contains a copy manager that is addressable through this logical unit. A 3 PC bit set to zero indicates that no copy manager is addressable through this logical unit.
    #[bit_byte(3, 3, 5, 5)]
    third_party_copy: bool,

    ///A PROTECT bit set to zero indicates that the logical unit does not support protection information. A PROTECT bit set to one indicates that the logical unit supports:
    /// a) type 1 protection, type 2 protection, or type 3 protection (see SBC-3); or
    /// b) logical block protection (see SSC-4).
    ///More information about the type of protection the logical unit supports is available in the SPT field (see 7.8.7).
    #[bit_byte(0, 0, 5, 5)]
    protect: bool,

    ///An Enclosure Services (ENCSERV) bit set to one indicates that the SCSI target device contains an embedded enclosure services component that is addressable through this logical unit. See SES-3 for details about enclosure services. An E NC S ERV bit set to zero indicates that no embedded enclosure services component is addressable through this logical unit.
    #[bit_byte(6, 6, 6, 6)]
    enclosure_services: bool,

    #[bit_byte(5, 5, 6, 6)]
    _vendor_specific: bool,

    ///A Multi Port (MULTIP) bit set to one indicates that this is a multi-port (two or more ports) SCSI target device and conforms to the SCSI multi-port device requirements found in the applicable standards (e.g., SAM-4, a SCSI transport protocol standard and possibly provisions of a command standard). A M ULTI P bit set to zero indicates that this SCSI target device has a single port and does not implement the multi-port requirements.
    #[bit_byte(4, 4, 6, 6)]
    multi_port: bool,

    /// SPI-5 only, reserved for all others
    #[bit_byte(0, 0, 6, 6)]
    _addr_16: bool,

    /// SPI-5 only, reserved for all others
    #[bit_byte(5, 5, 7, 7)]
    _wbus_16: bool,

    /// SPI-5 only, reserved for all others
    #[bit_byte(4, 4, 7, 7)]
    _sync: bool,

    ///The CMDQUE bit shall be set to one indicating that the logical unit supports the command management model defined in SAM-4.
    #[bit_byte(1, 1, 7, 7)]
    command_queue: bool,

    #[bit_byte(0, 0, 7, 7)]
    _vendor_specific2: bool,

    ///The T10 VENDOR IDENTIFICATION field contains eight bytes of left-aligned ASCII data (see 4.4.1) identifying the vendor of the logical unit. The T10 vendor identification shall be one assigned by INCITS. A list of assigned T10 vendor identifications is in Annex E and on the T10 web site (http://www.t10.org).
    #[bit_byte(0, 0, 8, 15)]
    pub vendor_identification: [u8; 8],

    ///The PRODUCT IDENTIFICATION field contains sixteen bytes of left-aligned ASCII data (see 4.4.1) defined by the vendor.
    #[bit_byte(0, 0, 16, 31)]
    pub product_identification: [u8; 16],

    ///The PRODUCT REVISION LEVEL field contains four bytes of left-aligned ASCII data defined by the vendor.
    #[bit_byte(0, 0, 32, 35)]
    pub product_revision_level: [u8; 4],

    #[bit_byte(0, 0, 36, 55)]
    _vendor_specific3: [u8; 20],

    /// SPI-5 only, reserved for all others
    #[bit_byte(3, 2, 56, 56)]
    _clocking: u8,

    /// SPI-5 only, reserved for all others
    #[bit_byte(1, 1, 56, 56)]
    _qas: bool,

    /// SPI-5 only, reserved for all others
    #[bit_byte(0, 0, 56, 56)]
    _ius: bool,

    ///The VERSION DESCRIPTOR fields provide for identifying up to eight standards to which the SCSI target device and/or logical unit claim conformance. The value in each VERSION DESCRIPTOR field shall be selected from table 144. All version descriptor values not listed in table 144 are reserved. Technical Committee T10 of INCITS maintains an electronic copy of the information in table 144 on its world wide web site (http://www.t10.org/). In the event that the T10 world wide web site is no longer active, access may be possible via the INCITS world wide web site (http://www.incits.org), the ANSI world wide web site (http://www.ansi.org), the IEC site (http://www.iec.ch/), the ISO site (http://www.iso.ch/), or the ISO/IEC JTC 1 web site (http://www.jtc1.org/). It is recommended that the first version descriptor be used for the SCSI architecture standard, followed by the physical transport standard if any, followed by the SCSI transport protocol standard, followed by the appropriate SPC-x version, followed by the device type command set, followed by a secondary command set if any.
    #[bit_byte(7, 0, 58, 59)]
    compliant_standard_1: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 60, 61)]
    compliant_standard_2: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 62, 63)]
    compliant_standard_3: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 64, 65)]
    compliant_standard_4: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 66, 67)]
    compliant_standard_5: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 68, 69)]
    compliant_standard_6: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 70, 71)]
    compliant_standard_7: u8, //VersionDescriptor,

    #[bit_byte(7, 0, 72, 73)]
    compliant_standard_8: u8, //VersionDescriptor,
}

impl InquiryResponse {
    pub const MINIMUM_SIZE: usize = 36;
}

impl Default for InquiryResponse {
    fn default() -> Self {
        let mut s = Self::new();

        s.set_removable_medium(true);
        //TODO(upstream): Work out why off by 1, docs say -4 but that's one byte too long
        //      It could be that sg_inq is adding 1 for some reason, the OS hasn't
        //      actually followed up with a longer request in real use.
        s.set_additional_length(0); // we have no additional length, 36 bytes works fine
        s.set_vendor_identification(&[ASCII_SPACE; 8]);
        s.set_product_identification(&[ASCII_SPACE; 16]);
        s.set_product_revision_level(&[ASCII_SPACE; 4]);
        s.set_compliant_standard_1(VersionDescriptor::SAM3NoVersionClaimed as _);
        s.set_compliant_standard_2(VersionDescriptor::SPC4NoVersionClaimed as _);
        s.set_compliant_standard_3(VersionDescriptor::SBC3NoVersionClaimed as _);

        s.set_peripheral_qualifier(Default::default());
        s.set_peripheral_device_type(Default::default());
        s.set_version(Default::default());
        s.set_normal_aca(Default::default());
        s.set_hierarchical_support(Default::default());
        s.set_response_data_format(Default::default());
        s.set_scc_supported(Default::default());
        s.set_access_controls_coordinator(Default::default());
        s.set_target_port_group_support(Default::default());
        s.set_third_party_copy(Default::default());
        s.set_protect(Default::default());
        s.set_enclosure_services(Default::default());
        s.set__vendor_specific(Default::default());
        s.set_multi_port(Default::default());
        s.set__addr_16(Default::default());
        s.set__wbus_16(Default::default());
        s.set__sync(Default::default());
        s.set_command_queue(Default::default());
        s.set__vendor_specific2(Default::default());
        s.set__vendor_specific3(&Default::default());
        s.set__clocking(Default::default());
        s.set__qas(Default::default());
        s.set__ius(Default::default());
        s.set_compliant_standard_4(Default::default());
        s.set_compliant_standard_5(Default::default());
        s.set_compliant_standard_6(Default::default());
        s.set_compliant_standard_7(Default::default());
        s.set_compliant_standard_8(Default::default());

        s
    }
}

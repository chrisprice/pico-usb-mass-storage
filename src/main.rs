#![no_std]
#![no_main]

use core::{any::Any, convert::Infallible, fmt::Display};

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_usb::{Builder, Config};
use embedded_io_async::ReadExactError;
use panic_probe as _;
use pico_usb_mass_storage::{bulk_only_transport::CommandError, usb_mass_storage::UsbMassStorage};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

static mut STORAGE: [u8; (BLOCKS * BLOCK_SIZE) as usize] = [0u8; (BLOCK_SIZE * BLOCKS) as usize];

static mut STATE: State = State {
    sense_key: None,
    sense_key_code: None,
    sense_qualifier: None,
};

const BLOCK_SIZE: u32 = 512;
const BLOCKS: u32 = 200;
const USB_PACKET_SIZE: u16 = 64; // 8,16,32,64
const MAX_LUN: u8 = 0; // max 0x0F

#[derive(Clone, Default)]
struct State {
    sense_key: Option<u8>,
    sense_key_code: Option<u8>,
    sense_qualifier: Option<u8>,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xabcd, 0xabcd);
    config.manufacturer = Some("Chris Price");
    config.product = Some("100k of your finest bytes");
    config.serial_number = Some("CP4096OYFB");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut mos_descriptor = [0; 0];
    let mut control_buf = [0; 64];

    let mut usb_mass_storage_state = pico_usb_mass_storage::usb_mass_storage::State::default();
    let mut scsi_handler = Handler();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut mos_descriptor,
        &mut control_buf,
    );

    let mut usb_mass_storage: UsbMassStorage<'_, _, NoopRawMutex> = UsbMassStorage::new(
        &mut usb_mass_storage_state,
        &mut builder,
        USB_PACKET_SIZE,
        MAX_LUN,
    );

    let mut usb = builder.build();
    let usb_fut = usb.run();

    let usb_mass_storage_fut = usb_mass_storage.run(&mut scsi_handler);

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, usb_mass_storage_fut).await;
}

struct Handler();

impl pico_usb_mass_storage::scsi::Handler for Handler {
    async fn read(
        &mut self,
        lba: u64,
        len: u64,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let lba = lba as u32;
        let len = len as u32;
        let start = (BLOCK_SIZE * lba) as usize;
        let end = start + (BLOCK_SIZE * len) as usize;
        for offset in (start..end).step_by(USB_PACKET_SIZE as usize) {
            writer
                .write_all(unsafe { &STORAGE[offset..offset + USB_PACKET_SIZE as usize] })
                .await?;
        }
        Ok(())
    }

    async fn write(
        &mut self,
        lba: u64,
        len: u64,
        reader: &mut impl embedded_io_async::Read<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        info!("write lba: {}, len: {}", lba, len);
        let lba = lba as u32;
        let len = len as u32;
        let start = (BLOCK_SIZE * lba) as usize;
        let end = start + (BLOCK_SIZE * len) as usize;
        for offset in (start..end).step_by(USB_PACKET_SIZE as usize) {
            reader
                .read_exact(unsafe { &mut STORAGE[offset..offset + USB_PACKET_SIZE as usize] })
                .await
                .map_err(|err| match err {
                    ReadExactError::UnexpectedEof => {
                        panic!("Unexpected EOF while writing to storage")
                    }
                    ReadExactError::Other(err) => err,
                })?;
        }

        log_fs();
        //log_storage();
        Ok(())
    }

    async fn inquiry(
        &mut self,
        _evpd: bool,
        _page_code: u8,
        _alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [
            0x00, // periph qualifier, periph device type
            0x80, // Removable
            0x04, // SPC-2 compliance
            0x02, // NormACA, HiSu, Response data format
            0x20, // 36 bytes in total
            0x00, // additional fields, none set
            0x00, // additional fields, none set
            0x00, // additional fields, none set
            b'C', b'H', b'R', b'I', b'S', b'P', b' ', b' ', // 8-byte T-10 vendor id
            b'1', b'0', b'0', b'k', b' ', b'o', b'f', b' ', b'y', b'o', b'u', b'r', b' ', b'f',
            b'i', b'n', // 16-byte product identification
            b'1', b'.', b'2', b'3', // 4-byte product revision
        ];
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn test_unit_ready(&mut self) -> Result<(), CommandError> {
        Ok(())
    }

    async fn request_sense(
        &mut self,
        _desc: bool,
        _alloc_len: u8,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [
            0x70, // RESPONSE CODE. Set to 70h for information on current errors
            0x00, // obsolete
            unsafe { STATE.sense_key.unwrap_or(0) }, // Bits 3..0: SENSE KEY. Contains information describing the error.
            0x00,
            0x00,
            0x00,
            0x00, // INFORMATION. Device-specific or command-specific information.
            0x00, // ADDITIONAL SENSE LENGTH.
            0x00,
            0x00,
            0x00,
            0x00,                                          // COMMAND-SPECIFIC INFORMATION
            unsafe { STATE.sense_key_code.unwrap_or(0) },  // ASC
            unsafe { STATE.sense_qualifier.unwrap_or(0) }, // ASCQ
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        writer.write_all(&data).await?;
        unsafe { STATE.reset() };
        Ok(())
    }

    async fn mode_sense6(
        &mut self,
        _dbd: bool,
        _page_control: pico_usb_mass_storage::scsi::command::PageControl,
        _page_code: u8,
        _subpage_code: u8,
        _alloc_len: u8,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [
            0x03, // number of bytes that follow
            0x00, // the media type is SBC
            0x00, // not write-protected, no cache-control bytes support
            0x00, // no mode-parameter block descriptors
        ];
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn mode_sense10(
        &mut self,
        _dbd: bool,
        _page_control: pico_usb_mass_storage::scsi::command::PageControl,
        _page_code: u8,
        _subpage_code: u8,
        _alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn read_capacity10(
        &mut self,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let mut data = [0u8; 8];
        let _ = &mut data[0..4].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
        let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn read_capacity16(
        &mut self,
        _alloc_len: u32,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let mut data = [0u8; 16];
        let _ = &mut data[0..8].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
        let _ = &mut data[8..12].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn read_format_capacities(
        &mut self,
        _alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let mut data = [0u8; 12];
        let _ = &mut data[0..4].copy_from_slice(&[
            0x00, 0x00, 0x00, 0x08, // capacity list length
        ]);
        let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCKS)); // number of blocks
        data[8] = 0x01; //unformatted media
        let block_length_be = u32::to_be_bytes(BLOCK_SIZE);
        data[9] = block_length_be[1];
        data[10] = block_length_be[2];
        data[11] = block_length_be[3];

        writer.write_all(&data).await?;
        Ok(())
    }

    async fn unknown(&mut self) -> Result<(), CommandError> {
        error!("Unknown SCSI command");
        unsafe {
            STATE.sense_key.replace(0x05); // illegal request Sense Key
            STATE.sense_key_code.replace(0x20); // Invalid command operation ASC
            STATE.sense_qualifier.replace(0x00); // Invalid command operation ASCQ
        }
        Err(CommandError::CommandFailed)
    }
}

#[derive(Clone)]
pub struct Partition {
    /// Partition Status
    pub p_status: u8,
    /// Start cylinder (Legacy CHS)
    pub p_cyl_begin: u8,
    /// Start head (Legacy CHS)
    pub p_head_begin: u8,
    /// Start sector (Legacy CHS)
    pub p_sect_begin: u8,
    /// Partition Type (DOS, Windows, BeOS, etc)
    pub p_type: u8,
    /// End cylinder (Legacy CHS)
    pub p_cyl_end: u8,
    /// End head (Legacy CHS)
    pub p_head_end: u8,
    /// End sector
    pub p_sect_end: u8,
    /// Logical block address to start of partition
    pub p_lba: u32,
    /// Number of sectors in partition
    pub p_size: u32,
}
impl core::fmt::Debug for Partition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Partition")
            .field("p_type", &format_args!("{:#02x}", self.p_type))
            .field("p_status", &format_args!("{:#02x}", self.p_status))
            .field("p_cyl_begin", &format_args!("{:#02x}", self.p_cyl_begin))
            .field("p_cyl_end", &format_args!("{:#02x}", self.p_cyl_end))
            .field("p_head_begin", &format_args!("{:#02x}", self.p_head_begin))
            .field("p_head_end", &format_args!("{:#02x}", self.p_head_end))
            .field("p_sect_begin", &format_args!("{:#02x}", self.p_sect_begin))
            .field("p_sect_end", &format_args!("{:#02x}", self.p_sect_end))
            .field("p_lba", &format_args!("{:#08x}", self.p_lba))
            .field("p_size", &format_args!("{:#08x}", self.p_size))
            .finish()
    }
}
pub struct ByteReader {
    position: u64,
}
impl ByteReader {
    fn new(position: u64) -> Self {
        Self { position }
    }
    fn read1(&mut self) -> u8 {
        let value = unsafe { STORAGE[self.position as usize] };
        self.position += 1;
        value
    }
    fn read4(&mut self) -> u32 {
        let value = unsafe {
            ((STORAGE[(self.position + 3) as usize] as u32) << 24)
                + ((STORAGE[(self.position + 2) as usize] as u32) << 16)
                + ((STORAGE[(self.position + 1) as usize] as u32) << 8)
                + ((STORAGE[(self.position + 0) as usize] as u32) << 0)
        };
        self.position += 4;
        value
    }
}
fn read_partition(index: u8) -> Partition {
    defmt::assert!(index < 4);

    let position: u64 = 446 + (16 * (index as u64));

    let mut byte_reader = ByteReader::new(position);

    Partition {
        p_status: byte_reader.read1(),
        p_head_begin: byte_reader.read1(),
        p_sect_begin: byte_reader.read1(),
        p_cyl_begin: byte_reader.read1(),
        p_type: byte_reader.read1(),
        p_head_end: byte_reader.read1(),
        p_sect_end: byte_reader.read1(),
        p_cyl_end: byte_reader.read1(),
        p_lba: byte_reader.read4(),
        p_size: byte_reader.read4(),
    }
}

fn log_fs() {
    let options = fatfs::FsOptions::new().update_accessed_date(false);

    let disk = MemFS::new();
    let fs = match fatfs::FileSystem::new(disk, options) {
        Ok(fatfs) => fatfs,
        Err(e) => {
            let s = match e {
                fatfs::Error::Io(e) => "unknown: MemFSError",
                fatfs::Error::UnexpectedEof => "UnexpectedEof",
                fatfs::Error::WriteZero => "WriteZero",
                fatfs::Error::InvalidInput => "InvalidInput",
                fatfs::Error::NotFound => "NotFound",
                fatfs::Error::AlreadyExists => "AlreadyExists",
                fatfs::Error::DirectoryIsNotEmpty => "DirectoryIsNotEmpty",
                fatfs::Error::CorruptedFileSystem => "CorruptedFileSystem",
                fatfs::Error::NotEnoughSpace => "NotEnoughSpace",
                fatfs::Error::InvalidFileNameLength => "InvalidFileNameLength",
                fatfs::Error::UnsupportedFileNameCharacter => "UnsupportedFileNameCharacter",
                _ => "Unknown",
            };
            error!("Error: {}", s);
            return;
        }
    };
    let fat_type = match fs.fat_type() {
        fatfs::FatType::Fat12 => "Fat12",
        fatfs::FatType::Fat16 => "Fat16",
        fatfs::FatType::Fat32 => "Fat32",
    };
    let volume_id = fs.volume_id();
    let volume_label: &str = core::str::from_utf8(fs.volume_label_as_bytes()).unwrap();

    info!(
        "type = {}, id = {}, label = {}",
        fat_type, volume_id, volume_label
    );
    let root = fs.root_dir();
    for d in root.iter().flatten() {
        let filename = core::str::from_utf8(d.short_file_name_as_bytes()).unwrap();
        let size = d.len();
        info!(
            "file name = \"{}\", size = {}, is_file: {}, is_dir: {}",
            filename,
            size,
            d.is_file(),
            d.is_dir()
        );
    }
}

struct MemFS {
    offset: u64,
}
impl MemFS {
    fn new() -> Self {
        Self { offset: 0 }
    }
}
impl fatfs::IoBase for MemFS {
    type Error = MemFSError;
}
impl fatfs::Read for MemFS {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let limit = usize::min(
            buf.len(),
            ((BLOCK_SIZE * BLOCKS) as u64 - self.offset)
                .try_into()
                .unwrap(),
        );

        unsafe {
            let src = STORAGE[self.offset as usize..self.offset as usize + limit].as_ptr();
            let dst = buf.as_mut_ptr();
            core::ptr::copy(src, dst, limit)
        };
        self.offset += limit as u64;
        Ok(limit)
    }
}
impl fatfs::Write for MemFS {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let limit = usize::min(
            buf.len(),
            ((BLOCK_SIZE * BLOCKS) as u64 - self.offset)
                .try_into()
                .unwrap(),
        );

        unsafe {
            //let mut buf = buf.as_ptr();
            let dst = STORAGE[self.offset as usize..self.offset as usize + limit].as_mut_ptr();
            core::ptr::copy(buf.as_ptr(), dst, limit)
        };
        self.offset += limit as u64;
        Ok(limit)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }
}
impl fatfs::Seek for MemFS {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            fatfs::SeekFrom::Start(p) => self.offset = p,
            fatfs::SeekFrom::End(p) => self.offset = ((BLOCKS * BLOCK_SIZE) as i64 - p) as u64,
            fatfs::SeekFrom::Current(p) => self.offset = (self.offset as i64 + p) as u64,
        }
        Ok(self.offset)
    }
}

#[derive(Debug)]
struct MemFSError {}
impl fatfs::IoError for MemFSError {
    fn is_interrupted(&self) -> bool {
        false
    }

    fn new_unexpected_eof_error() -> Self {
        unimplemented!()
    }

    fn new_write_zero_error() -> Self {
        unimplemented!()
    }
}

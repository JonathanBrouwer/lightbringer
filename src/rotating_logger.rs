use crate::make_static;
use core::cell::RefCell;
use core::fmt::Write;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use esp_println::println;
use log::{Level, Log, Metadata, Record};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

const BUFFER_SIZE: usize = 4196;
const MIN_LEVEL: Level = Level::Info;

pub struct RingBufferLogger {
    buffer: Mutex<CriticalSectionRawMutex, RefCell<RingBufferWrapper>>,
}

impl RingBufferLogger {
    pub fn init() -> &'static Self {
        let logger = make_static!(
            RingBufferLogger,
            Self {
                buffer: Mutex::new(RefCell::new(RingBufferWrapper(
                    ConstGenericRingBuffer::new()
                )))
            }
        );

        // Safety: The `make_static` macro above will panic if this code is ran more than once
        // We only call `set_logger` in this function so this is safe.
        unsafe {
            log::set_logger_racy(logger).unwrap();
            log::set_max_level_racy(MIN_LEVEL.to_level_filter());
        }

        logger
    }

    pub fn get_logs(&self) -> heapless::Vec<u8, BUFFER_SIZE> {
        self.buffer
            .lock(|buffer| buffer.borrow().0.iter().cloned().collect())
    }
}

impl Log for RingBufferLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= MIN_LEVEL
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let module_path = record.module_path().unwrap_or("???");
        println!("[{}] {} - {}\n", record.level(), module_path, record.args());
        self.buffer.lock(|buffer| {
            let mut buffer = buffer.borrow_mut();
            writeln!(
                buffer,
                "[{}] {} - {}",
                record.level(),
                module_path,
                record.args()
            )
            .unwrap();
        });
    }

    fn flush(&self) {}
}

struct RingBufferWrapper(ConstGenericRingBuffer<u8, BUFFER_SIZE>);

impl Write for RingBufferWrapper {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.extend(s.as_bytes().iter().cloned());
        Ok(())
    }
}

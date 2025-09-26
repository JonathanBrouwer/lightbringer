#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use picoserve::make_static;

pub mod color_storage;
pub mod http;
pub mod leds;
pub mod light_state;
pub mod partitions;
pub mod rotating_logger;
pub mod value_synchronizer;
pub mod wifi;
pub mod web_app;
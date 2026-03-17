use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use bollard::secret::{BuildInfo, CreateImageInfo};
use dashmap::DashMap;
use owo_colors::OwoColorize;

use crate::cli_memory;

#[derive(Clone, Copy)]
pub enum ServiceColor {
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
}

impl ServiceColor {
    fn paint(&self, text: &str) -> String {
        match self {
            ServiceColor::Green => text.green().to_string(),
            ServiceColor::Blue => text.blue().to_string(),
            ServiceColor::Yellow => text.yellow().to_string(),
            ServiceColor::Cyan => text.cyan().to_string(),
            ServiceColor::Magenta => text.magenta().to_string(),
        }
    }
}

const COLORS: [ServiceColor; 5] = [
    ServiceColor::Green,
    ServiceColor::Blue,
    ServiceColor::Yellow,
    ServiceColor::Cyan,
    ServiceColor::Magenta,
];

pub fn service_logs(service_name: &str, service_log: BuildInfo) {
    let a= String::from("a");
    let b = a.bright_green();
    println!(
        "{} -> {:?}",
        hash_to_colored_string(service_name),
        service_log
    );
}

pub fn service_logs_messages(service_name: &str, service_log: &str) {
    println!(
        "{} -> {:?}",
        hash_to_colored_string(service_name),
        service_log
    );
}

pub fn service_started(service_name: &str, service_log: String) {
    println!(
        "{} -> {:?}",
        hash_to_colored_string(service_name),
        service_log
    );
}

pub fn show_service_error_logs(
    service_name: &str,
    error_message: &str,
) {
    println!(
        "{} -> {:?}",
        hash_to_colored_string(service_name),
        error_message
    );
}

pub fn show_pulled_image_specific_logs(
    image_name: &str,
    data: CreateImageInfo,
) {
    println!("{} -> {:?}", hash_to_colored_string(image_name), data);
}

/**
 * red delete message
 */
pub fn service_stop_or_delete_message(container_id: &str, service_log: &str) {
    println!("{} -> {:?}", container_id.red(), service_log);
}

/**
 * green general message
 */
pub fn general_message(id: &str, message: &str) {
    println!("{} -> {:?}", id.green(), message);
}


/**
 * green general message
 */
pub fn general_error_message(id: &str, message: &str) {
    println!("{} -> {:?}", id.red(), message);
}

/**
 * this function wil hash the string into a u64 bits number
 * thenw e will % that number with COLORS.le() , so that number will comese in between 0 to len_of_COLORS_arry -1
 * arrray return an enum value ,and then we will call enum.paint to get colored String
 *
// auth_Service => 98347234982374
 */
pub fn hash_to_colored_string(to_be_colored_string: &str) -> String {
    let mut hasher = DefaultHasher::new();
    to_be_colored_string.hash(&mut hasher);
    let idx = (hasher.finish() as usize) % COLORS.len();

    let str_color = COLORS[idx];

    let colored_str = str_color.paint(to_be_colored_string);

    return colored_str;
}

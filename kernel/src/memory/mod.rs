pub mod pmm;
pub mod vmm;
pub mod paging;

pub fn init() {
    std::println!("[MEMORY] Init...");
    pmm::init();
    vmm::init();
    std::println!("[MEMORY] Init Done.");
}

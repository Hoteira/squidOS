use crate::memory::{paging, pmm};
use core::arch::{asm, naked_asm};

#[allow(dead_code)]
const STACK_SIZE: u64 = 64 * 1024;
pub(crate) const MAX_TASKS: usize = 125;


#[derive(Copy, Clone, Debug)]
#[repr(C, align(16))]
pub struct Task {
    pub fpu_state: [u8; 512],
    pub kernel_stack: u64,
    pub stack: u64,
    pub cpu_state_ptr: u64,
    pub state: TaskState,
    pub pml4_phys: u64,
    pub fd_table: [i16; 16],
    pub exit_code: u64,
    pub wake_ticks: u64,
    pub name: [u8; 32],
    pub cwd: [u8; 128],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TaskState {
    Null,
    Reserved,
    Ready,
    Zombie,
    Sleeping,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct CPUState {
    pub(crate) r15: u64,
    pub(crate) r14: u64,
    pub(crate) r13: u64,
    pub(crate) r12: u64,
    pub(crate) r11: u64,
    pub(crate) r10: u64,
    pub(crate) r9: u64,
    pub(crate) r8: u64,
    pub(crate) rdi: u64,
    pub(crate) rsi: u64,
    pub(crate) rdx: u64,
    pub(crate) rcx: u64,
    pub(crate) rbx: u64,
    pub(crate) rax: u64,
    pub(crate) rbp: u64,

    pub(crate) rip: u64,
    pub(crate) cs: u64,
    pub(crate) rflags: u64,
    pub(crate) rsp: u64,
    pub(crate) ss: u64,
}

pub(crate) static NULL_TASK: Task = Task {
    fpu_state: [0; 512],
    stack: 0,
    kernel_stack: 0,
    cpu_state_ptr: 0,
    state: TaskState::Null,
    pml4_phys: 0,
    fd_table: [-1; 16],
    exit_code: 0,
    wake_ticks: 0,
    name: [0; 32],
    cwd: [0; 128],
};

impl Task {
    pub fn init(&mut self, entry_point: u64, args: Option<&[u64]>, name: &[u8]) {
        self.state = TaskState::Ready;
        self.fpu_state = [0; 512];
        self.fd_table = [-1; 16];
        self.exit_code = 0;
        self.wake_ticks = 0;
        self.name = [0; 32];
        let len = core::cmp::min(name.len(), 32);
        self.name[..len].copy_from_slice(&name[..len]);

        self.cwd = [0; 128];
        let root = b"@0xE0/";
        self.cwd[..root.len()].copy_from_slice(root);

        self.fpu_state[0] = 0x7F;

        self.fpu_state[1] = 0x03;

        self.fpu_state[24] = 0x80;
        self.fpu_state[25] = 0x1F;

        unsafe {
            self.pml4_phys = (*(&raw const crate::boot::BOOT_INFO)).pml4;
        }

        let stack_pages = (STACK_SIZE / 4096) as usize;
        self.stack = pmm::allocate_frames(stack_pages, 0).expect("Task init: OOM");

        let stack_top = self.stack + STACK_SIZE;
        self.kernel_stack = stack_top;

        let state_size = core::mem::size_of::<CPUState>();
        let state_ptr = (stack_top - state_size as u64) as *mut CPUState;
        self.cpu_state_ptr = state_ptr as u64;

        let mut arg_count = 0;
        if args.is_some() {
            arg_count = core::cmp::min(args.unwrap().len(), 4);
        }

        unsafe {
            (*state_ptr).rax = 0;
            (*state_ptr).rbx = if arg_count > 0 { args.unwrap()[0] } else { 0 };
            (*state_ptr).rcx = if arg_count > 1 { args.unwrap()[1] } else { 0 };
            (*state_ptr).rdx = if arg_count > 2 { args.unwrap()[2] } else { 0 };
            (*state_ptr).rsi = if arg_count > 3 { args.unwrap()[3] } else { 0 };

            (*state_ptr).rdi = 0;
            (*state_ptr).rbp = 0;
            (*state_ptr).rsp = stack_top;
            (*state_ptr).rip = entry_point;
            (*state_ptr).cs = 0x28;
            (*state_ptr).rflags = 0x202;
            (*state_ptr).ss = 0x10;
        }
    }

    #[allow(dead_code)]
    pub fn init_user(&mut self, entry_point: u64, pml4_phys: u64, args: Option<&[u64]>, pid: u64, fd_table: Option<[i16; 16]>, name: &[u8]) -> Result<(), pmm::FrameError> {
        self.fpu_state = [0; 512];
        self.fd_table = fd_table.unwrap_or([-1; 16]);
        self.exit_code = 0;
        self.wake_ticks = 0;
        self.name = [0; 32];
        let len = core::cmp::min(name.len(), 32);
        self.name[..len].copy_from_slice(&name[..len]);

        self.cwd = [0; 128];
        let root = b"@0xE0/";
        self.cwd[..root.len()].copy_from_slice(root);

        self.fpu_state[0] = 0x7F;
        self.fpu_state[1] = 0x03;
        self.fpu_state[24] = 0x80;
        self.fpu_state[25] = 0x1F;

        self.pml4_phys = pml4_phys;


        let k_frame = match pmm::allocate_frames(16, pid) {
            Some(addr) => addr,
            None => return Err(pmm::FrameError::NoMemory),
        };
        self.kernel_stack = k_frame + 4096 * 16;

        let stack_pages = (STACK_SIZE / 4096) as usize;
        let u_frame = match pmm::allocate_frames(stack_pages, pid) {
            Some(addr) => addr,
            None => {
                pmm::free_frame(k_frame);
                return Err(pmm::FrameError::NoMemory);
            }
        };
        self.stack = u_frame;

        let u_stack_top = u_frame + STACK_SIZE;

        let state_size = core::mem::size_of::<CPUState>();
        let state_ptr = (self.kernel_stack - state_size as u64) as *mut CPUState;
        self.cpu_state_ptr = state_ptr as u64;

        let mut arg_count = 0;
        if args.is_some() {
            arg_count = core::cmp::min(args.unwrap().len(), 4);
        }

        unsafe {
            // Setup a clean stack for System V ABI:
            // [u_stack_top - 128..u_stack_top] : Program Name (Strings)
            // [u_stack_top - 160..u_stack_top-128] : argv array pointers
            // [u_stack_top - 168] : argc (The entry RSP)
            
            let name_len = core::cmp::min(name.len(), 63);
            let name_addr = u_stack_top - 128; 
            let name_ptr = name_addr as *mut u8;
            core::ptr::copy_nonoverlapping(name.as_ptr(), name_ptr, name_len);
            *name_ptr.add(name_len) = 0;

            let stack_ptrs = u_stack_top - 160;
            let stack_ptr_u64 = stack_ptrs as *mut u64;
            stack_ptr_u64.add(0).write(name_addr); // argv[0]
            stack_ptr_u64.add(1).write(0);         // argv[1] (NULL)
            stack_ptr_u64.add(2).write(0);         // envp[0] (NULL)

            let argc_addr = u_stack_top - 168;
            *(argc_addr as *mut u64) = 1;          // argc = 1

            (*state_ptr).rax = 0;
            (*state_ptr).rbx = if arg_count > 0 { args.unwrap()[0] } else { 0 };
            (*state_ptr).rcx = if arg_count > 1 { args.unwrap()[1] } else { 0 };
            (*state_ptr).rdx = if arg_count > 2 { args.unwrap()[2] } else { 0 };
            (*state_ptr).rsi = if arg_count > 3 { args.unwrap()[3] } else { 0 };

            (*state_ptr).rdi = 0;
            (*state_ptr).rbp = 0;

            (*state_ptr).rip = entry_point;
            (*state_ptr).cs = 0x33;
            (*state_ptr).rflags = 0x202;
            (*state_ptr).rsp = argc_addr; // 16n + 8 alignment (since u_stack_top is 4096n)
            (*state_ptr).ss = 0x23;
        }

        self.state = TaskState::Ready;
        Ok(())
    }
}

pub struct TaskManager {
    pub tasks: [Task; MAX_TASKS],
    task_count: usize,
    pub(crate) current_task: isize,
}

#[allow(dead_code)]
pub struct LockedTaskManager {
    inner: std::sync::Mutex<TaskManager>,
}

pub static TASK_MANAGER: std::sync::Mutex<TaskManager> =
    std::sync::Mutex::new(TaskManager {
        tasks: [NULL_TASK; MAX_TASKS],
        task_count: 0,
        current_task: -1,
    });

#[unsafe(no_mangle)]
pub static mut KERNEL_STACK_PTR: u64 = 0;

#[unsafe(no_mangle)]
pub static mut SCRATCH: u64 = 0;

impl TaskManager {
    pub fn init(&mut self) {
        self.add_task(idle as u64, None, b"idle");
    }

    pub fn add_task(&mut self, entry_point: u64, args: Option<&[u64]>, name: &[u8]) {
        if self.task_count < MAX_TASKS {
            let free_slot = self.get_free_slot();
            self.tasks[free_slot].init(entry_point, args, name);
            self.task_count += 1;
        }
    }

    pub fn current_task_idx(&self) -> Option<usize> {
        if self.current_task >= 0 {
            Some(self.current_task as usize)
        } else {
            None
        }
    }

    pub fn reserve_pid(&mut self) -> Result<usize, pmm::FrameError> {
        if let Some(slot) = self.tasks.iter().position(|t| t.state == TaskState::Null) {
            self.tasks[slot].state = TaskState::Reserved;
            self.task_count += 1;
            Ok(slot)
        } else {
            Err(pmm::FrameError::NoMemory)
        }
    }

    pub fn kill_process(&mut self, pid: u64) {
        if pid < MAX_TASKS as u64 {
            let task = &mut self.tasks[pid as usize];
            if task.state != TaskState::Null && task.state != TaskState::Zombie && task.state != TaskState::Null {
                task.exit_code = 0xDEAD;
                task.state = TaskState::Zombie;


                unsafe {
                    (*(&raw mut crate::window_manager::composer::COMPOSER)).remove_windows_by_pid(pid);
                }


                for i in 0..16 {
                    let global = task.fd_table[i];
                    if global != -1 {
                        crate::fs::vfs::close_file(global as usize);
                        task.fd_table[i] = -1;
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn init_user_task(&mut self, slot: usize, entry_point: u64, pml4_phys: u64, args: Option<&[u64]>, fd_table: Option<[i16; 16]>, name: &[u8]) -> Result<(), pmm::FrameError> {
        if slot >= MAX_TASKS { return Err(pmm::FrameError::IndexOutOfBounds); }


        match self.tasks[slot].init_user(entry_point, pml4_phys, args, slot as u64, fd_table, name) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.tasks[slot].state = TaskState::Null;
                self.task_count -= 1;
                Err(e)
            }
        }
    }

    pub fn schedule(&mut self, cpu_state: *mut CPUState) -> (*mut CPUState, u64, u64) {
        unsafe {
            for task in self.tasks.iter_mut() {
                if task.state == TaskState::Sleeping && SYSTEM_TICKS >= task.wake_ticks {
                    task.state = TaskState::Ready;
                }
            }
        }

        if self.current_task >= 0 {
            self.tasks[self.current_task as usize].cpu_state_ptr = cpu_state as u64;
        }

        self.current_task = self.get_next_task();
        if self.current_task < 0 {
            return (cpu_state, 0, 0);
        }

        (
            self.tasks[self.current_task as usize].cpu_state_ptr as *mut CPUState,
            self.tasks[self.current_task as usize].kernel_stack,
            self.tasks[self.current_task as usize].pml4_phys,
        )
    }

    pub fn get_next_task(&self) -> isize {
        let mut i = self.current_task + 1;
        let limit = MAX_TASKS as isize;

        let start_i = i;

        loop {
            if i >= limit {
                i = 0;
            }

            if self.tasks[i as usize].state == TaskState::Ready {
                return i;
            }

            i += 1;
            if i == start_i {
                break;
            }
        }

        if self.tasks[0].state == TaskState::Ready {
            0
        } else {
            -1
        }
    }

    fn get_free_slot(&self) -> usize {
        for i in 0..MAX_TASKS {
            if self.tasks[i].state == TaskState::Null {
                return i;
            }
        }

        panic!("No free slots available!");
    }
}

fn idle() {
    loop {
        unsafe { asm!("hlt") };
    }
}

#[unsafe(naked)]
pub extern "C" fn timer_handler() {
    unsafe {
        naked_asm!(
            "push rbp",
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            "mov rdi, rsp",
            "call switch_timer",

            "mov rsp, rax",

            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "pop rbp",

            "iretq",
        );
    }
}

#[unsafe(naked)]
pub extern "C" fn yield_handler() {
    unsafe {
        naked_asm!(
            "push rbp",
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            "mov rdi, rsp",
            "call switch_yield",

            "mov rsp, rax",

            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "pop rbp",

            "iretq",
        );
    }
}


#[unsafe(no_mangle)]
pub static mut SYSTEM_TICKS: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn switch_timer(rsp: u64) -> u64 {
    unsafe { common_switch(rsp, true) }
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_yield(rsp: u64) -> u64 {
    unsafe { common_switch(rsp, false) }
}

unsafe fn common_switch(rsp: u64, is_timer: bool) -> u64 {
    unsafe {
        if is_timer {
            SYSTEM_TICKS = SYSTEM_TICKS.wrapping_add(10);
        }
        let mut tm = TASK_MANAGER.int_lock();


        if tm.current_task >= 0 {
            let index = tm.current_task as usize;
            let task_ptr = &mut tm.tasks[index] as *mut Task;
            let fpu_ptr = (*task_ptr).fpu_state.as_mut_ptr();
            asm!("fxsave [{}]", in(reg) fpu_ptr);
        }

        let (new_state, k_stack, _pml4_phys) = tm.schedule(rsp as *mut CPUState);


        if tm.current_task >= 0 {
            let index = tm.current_task as usize;
            let task_ptr = &tm.tasks[index] as *const Task;
            let fpu_ptr = (*task_ptr).fpu_state.as_ptr();
            asm!("fxrstor [{}]", in(reg) fpu_ptr);
        }

        if k_stack != 0 {
            crate::tss::set_tss(k_stack);
            KERNEL_STACK_PTR = k_stack;
        }

        if is_timer {
            (*(&raw const crate::interrupts::pic::PICS)).end_interrupt(crate::interrupts::exceptions::TIMER_INT);
        }

        new_state as u64
    }
}
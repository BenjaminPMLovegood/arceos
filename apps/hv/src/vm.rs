use axvm;
use axprocess;

pub fn boot_first_vm(hart_id: usize) {
    axvm::config_boot_first_vm(hart_id);

    println!("config_boot_first_vm ok");

    loop {
        // if unsafe { axprocess::wait_pid(now_process_id, &mut exit_code as *mut i32) }.is_ok() {
        //     break Some(exit_code);
        // }

        axprocess::yield_now_task();
    };
}
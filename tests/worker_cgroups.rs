// mod utils;
// use claim::assert_ok;
// use iron_exec::job::{CgroupConfig, Command};
// use utils::app::TestApp;
// use uuid::Uuid;

// #[test]
// pub fn test_cgroup_success() {
//     let mut app = TestApp::new();

//     let test_cases = [
//         (
//             Some(String::from("10000 100000")),
//             None,
//             None,
//             None,
//             None,
//             None,
//             "has cpu.max defined",
//         ),
//         // (
//         //     None,
//         //     Some(75 as u16),
//         //     None,
//         //     None,
//         //     None,
//         //     None,
//         //     "has cpu.weight defined",
//         // ),
//         // (
//         //     None,
//         //     None,
//         //     Some(155 as u32),
//         //     None,
//         //     None,
//         //     None,
//         //     "has memory.max defined",
//         // ),
//         // (
//         //     None,
//         //     None,
//         //     None,
//         //     Some(75 as u16),
//         //     None,
//         //     None,
//         //     "has memory.weight defined",
//         // ),
//         // (
//         //     None,
//         //     None,
//         //     None,
//         //     None,
//         //     Some(155 as u32),
//         //     None,
//         //     "has io.max defined",
//         // ),
//         // (
//         //     None,
//         //     None,
//         //     None,
//         //     None,
//         //     None,
//         //     Some(75 as u16),
//         //     "has io.weight defined",
//         // ),
//     ];

//     for (cpu_max, cpu_weight, memory_max, memory_weight, io_max, io_weight, error_case) in
//         test_cases
//     {
//         let cgroup_config = CgroupConfig::new(
//             cpu_max,
//             cpu_weight,
//             memory_max,
//             memory_weight,
//             io_max,
//             io_weight,
//             // Some("./tests/cgroup"),
//         );

//         let (job_id, job_handle) = assert_ok!(
//             app.worker.start(
//                 // Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
//                 Command::new("sh", &["./tests/scripts/infinite_loop.sh"]),
//                 Some(cgroup_config),
//                 Uuid::new_v4(),
//             ),
//             "failed to start job with cgroup that {}",
//             error_case,
//         );

//         // panic!("STOP");
//         job_handle.join().unwrap();
//     }
// }

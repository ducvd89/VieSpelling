//! Lõi ứng dụng, dùng chung cho bản có cửa sổ và bản dòng lệnh.
//!
//! Tách ra thành thư viện để bản dòng lệnh chạy **đúng đường đi** mà bản cửa sổ
//! chạy. Nếu mỗi bên tự nối lại các tầng thì đo đạc trên bản dòng lệnh không
//! nói lên điều gì về bản người dùng thật sự bấm.

pub mod bao_cao;
pub mod cai_dat;
pub mod nhat_ky;
pub mod tai_cuda;
pub mod xu_ly;

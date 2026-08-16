//! Lõi kiểm và sửa lỗi chính tả tiếng Việt.
//!
//! Chia tầng theo **độ chắc chắn**, vì ứng dụng tự sửa rồi mới báo cáo — sửa
//! nhầm tốn của người dùng nhiều hơn bỏ sót:
//!
//! 1. [`chuan_hoa`] — Unicode, ký tự vô hình, khoảng trắng, dấu câu. Không đụng
//!    tới chữ nghĩa nên sửa vô điều kiện.
//! 2. [`dau_thanh`] — dấu thanh đặt sai nguyên âm, và chuyện kiểu cũ/kiểu mới.
//! 3. [`am_tiet`] — tiếng sai cấu tạo. Bắt được thì biết sai ở đâu.
//! 4. [`de_nham`] — cặp dễ nhầm, tra bảng.
//! 5. [`ung_vien`] — từ tiếng sai sinh ra các cách sửa có lý.
//! 6. Chấm điểm ứng viên bằng mô hình ngôn ngữ (crate `mohinh`) — tầng duy nhất
//!    biết ngữ cảnh, và là cái chặn cuối trước khi thay chữ của tác giả.
//!
//! # Ngoài phạm vi: bảng mã cũ
//!
//! Sách từ những năm 2000 có khi còn nằm ở **TCVN3 hoặc VNI** — cả file đặc
//! những ký tự như `¬ ® Ý ß` ở chỗ đáng lẽ là chữ có dấu. Ứng dụng này **cố ý
//! không xử lý** chúng: nó chỉ nhận EPUB Unicode.
//!
//! Đây là chuyện an toàn chứ không phải lười. Nhận diện bảng mã cũ là đoán, mà
//! đoán sai thì phép chuyển đổi ánh xạ sai toàn bộ ký tự của cả cuốn sách — hỏng
//! nặng hơn nhiều so với mọi lỗi mà ứng dụng này chữa. File bảng mã cũ đưa vào
//! đây thì **đi qua nguyên vẹn**: mọi tầng đều thấy chúng không phải chữ tiếng
//! Việt nên không đụng tới. Muốn chuyển thì dùng công cụ chuyên làm việc ấy
//! trước, rồi mới đưa vào đây.

pub mod am_tiet;
pub mod chuan_hoa;
pub mod dau_thanh;
pub mod de_nham;
pub mod doi_chieu;
pub mod soat;
pub mod sua;
pub mod tach_tu;
pub mod tu_dien;
pub mod ung_vien;

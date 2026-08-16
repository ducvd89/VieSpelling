//! Kênh báo tiến độ và nhật ký từ luồng nền về giao diện.
//!
//! Việc xử lý một cuốn sách dài chạy hàng phút và đi qua năm sáu giai đoạn khác
//! hẳn nhau. Một thanh tiến trình trơ trọi không nói được nó đang ở đâu — mà
//! đúng lúc nó đứng im lâu nhất (nạp mô hình 5 GB, chấm điểm hàng nghìn câu)
//! thì người dùng cần biết nhất là nó có còn sống không.
//!
//! Nên có hai kênh, và chúng trả lời hai câu khác nhau:
//!
//! - [`Tin::TienDo`] — *còn bao lâu nữa*. Ghi đè lên nhau, chỉ giữ cái mới nhất.
//! - [`Tin::Ghi`] — *nó vừa làm gì*. Cộng dồn thành một dòng thời gian đọc được.

use std::time::Instant;

/// Mức của một dòng nhật ký. Giao diện lọc và tô màu theo đây.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Muc {
    /// Từng chỗ sửa một. Rất nhiều — mặc định ẩn.
    ChiTiet,
    /// Mốc của một giai đoạn: mở sách xong, nạp mô hình xong, ghi file xong.
    Buoc,
    /// Có chuyện đáng để ý nhưng không dừng được việc.
    CanhBao,
}

impl Muc {
    pub fn dau(self) -> &'static str {
        match self {
            Muc::ChiTiet => "  ",
            Muc::Buoc => "▸ ",
            Muc::CanhBao => "! ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Dong {
    pub muc: Muc,
    pub chu: String,
    /// Giây kể từ lúc bắt đầu. Đây là thứ biến nhật ký thành công cụ đo: nhìn
    /// khoảng cách giữa hai dòng là biết bước nào ăn hết thời gian.
    pub giay: f32,
}

#[derive(Debug, Clone)]
pub enum Tin {
    TienDo { ty_le: f32, mo_ta: String },
    Ghi(Dong),
}

/// Bộ báo cáo mà tầng xử lý cầm. Nó không biết đầu kia là cửa sổ hay dòng lệnh.
pub struct Bao<'a> {
    gui: &'a mut dyn FnMut(Tin),
    bat_dau: Instant,
}

impl<'a> Bao<'a> {
    pub fn moi(gui: &'a mut dyn FnMut(Tin)) -> Bao<'a> {
        Bao { gui, bat_dau: Instant::now() }
    }

    pub fn tien_do(&mut self, ty_le: f32, mo_ta: impl Into<String>) {
        (self.gui)(Tin::TienDo { ty_le, mo_ta: mo_ta.into() });
    }

    pub fn ghi(&mut self, muc: Muc, chu: impl Into<String>) {
        (self.gui)(Tin::Ghi(Dong {
            muc,
            chu: chu.into(),
            giay: self.bat_dau.elapsed().as_secs_f32(),
        }));
    }

    pub fn buoc(&mut self, chu: impl Into<String>) {
        self.ghi(Muc::Buoc, chu);
    }

    pub fn chi_tiet(&mut self, chu: impl Into<String>) {
        self.ghi(Muc::ChiTiet, chu);
    }

    pub fn canh_bao(&mut self, chu: impl Into<String>) {
        self.ghi(Muc::CanhBao, chu);
    }

    pub fn giay(&self) -> f32 {
        self.bat_dau.elapsed().as_secs_f32()
    }
}

//! Điều phối các tầng dò lỗi trên một đoạn văn.
//!
//! **Thứ tự các tầng không đổi chỗ được.** Chuẩn hoá Unicode phải chạy trước
//! mọi thứ, vì một chữ `ế` gõ rời (e + mũ + sắc) trông giống hệt chữ đúng nhưng
//! trượt mọi phép so; chạy bộ kiểm âm tiết trước khi dựng lại NFC thì cả đoạn
//! bị báo sai. Dọn khoảng trắng phải chạy **sau** dấu câu, vì luật dấu câu chèn
//! thêm khoảng trắng và có thể chèn ra chỗ đã có sẵn một cái.
//!
//! Mỗi tầng nhận chuỗi và trả về **danh sách phép sửa trên chính chuỗi ấy**,
//! rồi bộ điều phối áp xong mới chạy tầng sau. Chạy hết mọi tầng trên cùng một
//! chuỗi rồi áp một lượt thì nhanh hơn, nhưng các phép sửa sẽ chồng lên nhau ở
//! những chỗ hai tầng cùng nhìn thấy vấn đề, và cái bị bỏ lại là cái ngẫu nhiên.

use crate::chuan_hoa::{self, CaiDat};
use crate::dau_thanh::{self, Kieu};
use crate::de_nham;
use crate::sua::{ap_dung, DoChac, Loai, SuaDoi};
use crate::tach_tu::{self, DangTu};
use crate::tu_dien;
use crate::{am_tiet, ung_vien};
use std::ops::Range;

/// Một chỗ bộ dò không tự quyết được — phải hỏi mô hình ngôn ngữ.
#[derive(Debug, Clone)]
pub struct ChoXet {
    /// Vị trí trong chuỗi **kết quả** của [`KetQua::chu`].
    pub pham_vi: Range<usize>,
    pub goc: String,
    /// Các cách sửa, đã xếp hạng sơ bộ. Bản gốc **không** nằm trong đây.
    pub ung_vien: Vec<String>,
    pub loai: Loai,
    pub ly_do: String,
    /// Ứng viên đứng đầu là **ứng viên duy nhất** ghép được với hàng xóm thành
    /// một từ có trong từ điển.
    ///
    /// Đây là bằng chứng mạnh hơn mọi điểm số: `chúg ta` thì chỉ `chúng` ghép
    /// được thành `chúng ta`, và không cần hỏi thêm ai. Bật cờ này thì phép sửa
    /// tự áp được ngay cả khi không có mô hình ngôn ngữ.
    pub chac_nho_tu_ghep: bool,
}

#[derive(Debug, Clone, Default)]
pub struct KetQua {
    pub chu: String,
    pub da_sua: Vec<SuaDoi>,
    pub cho_xet: Vec<ChoXet>,
}

/// Máy chấm điểm câu. Cài đặt thật nằm ở crate `mohinh`; để ở đây dạng trait
/// nên lõi chính tả không phụ thuộc vào llama.cpp và chạy test không cần mô hình.
pub trait ChamDiem {
    /// Điểm tự nhiên của câu — **càng cao càng tự nhiên**. Chỉ dùng để **so**
    /// các câu gần giống hệt nhau, nên thang đo tuyệt đối không quan trọng, miễn
    /// nó nhất quán giữa các lần gọi.
    fn cham(&self, cau: &str) -> f32;
}

#[derive(Debug, Clone)]
pub struct TuyChon {
    pub chuan_hoa: CaiDat,
    /// Kéo cả sách về một kiểu đặt dấu thanh.
    pub nhat_quan_dau_thanh: bool,
    /// Sửa tiếng sai cấu tạo.
    pub am_tiet_sai: bool,
    /// Dò cặp dễ nhầm.
    pub de_nham: bool,
    /// Đụng cả vào chữ không dấu. **Mặc định tắt** — không phân được `khong`
    /// (tiếng Việt thiếu dấu) với `window` (tiếng Anh) nếu chỉ nhìn một tiếng,
    /// mà sách dịch thì đầy tên riêng nước ngoài.
    pub chu_khong_dau: bool,
    /// Mô hình phải chấm hơn bao nhiêu thì mới được đổi, tính bằng
    /// **nats/ký tự**.
    ///
    /// Đây là **cái van an toàn của cả ứng dụng**, và nó chặn hai chỗ: cách sửa
    /// phải hơn bản gốc quá mức này, **và** nếu mô hình muốn lật ngược thứ tự
    /// mà tầng luật đã xếp thì cũng phải hơn quá mức này.
    ///
    /// Đơn vị là mỗi **ký tự** chứ không phải mỗi token — xem
    /// `mohinh::MoHinh::cham_that` về lý do. Nên con số nhỏ hơn hẳn mức quen
    /// thuộc: một token tiếng Việt cỡ bốn năm ký tự.
    pub nguong_mo_hinh: f32,
}

impl Default for TuyChon {
    fn default() -> Self {
        TuyChon {
            chuan_hoa: CaiDat::default(),
            nhat_quan_dau_thanh: true,
            am_tiet_sai: true,
            de_nham: true,
            chu_khong_dau: false,
            nguong_mo_hinh: 0.03,
        }
    }
}

pub struct BoSoat {
    pub tuy_chon: TuyChon,
    bang_de_nham: de_nham::Bang,
    kieu: Kieu,
    ten_rieng: std::collections::HashSet<String>,
}

impl BoSoat {
    pub fn moi(tuy_chon: TuyChon, kieu: Kieu) -> BoSoat {
        BoSoat {
            tuy_chon,
            bang_de_nham: de_nham::Bang::nap(),
            kieu,
            ten_rieng: Default::default(),
        }
    }

    /// Danh sách tên riêng đếm được từ chính cuốn sách — xem [`gom_ten_rieng`].
    pub fn voi_ten_rieng(mut self, ten: std::collections::HashSet<String>) -> BoSoat {
        self.ten_rieng = ten;
        self
    }

    /// Chạy mọi tầng không cần ngữ cảnh trên một đoạn.
    pub fn soat(&self, van_ban: &str) -> KetQua {
        let tc = &self.tuy_chon;
        let mut kq = KetQua::default();
        let mut chu = van_ban.to_string();

        // Tầng 1 — Unicode. Phải đứng đầu.
        let mut s = chuan_hoa::soat_unicode(&chu, &tc.chuan_hoa);
        chu = ghi_nhan(&chu, &mut s, &mut kq);
        if tc.chuan_hoa.unicode {
            if let Some(nfc) = chuan_hoa::dung_lai_nfc(&chu) {
                // Ghi **từng chữ đổi**, không ghi cả đoạn làm một phép sửa.
                // NFC gộp ký tự nên đoạn nào có một chữ gõ rời là cả đoạn khác
                // đi; ghi thô thì báo cáo đầy những dòng dài hàng trăm chữ mà
                // hai vế nhìn giống hệt nhau — đúng nhưng đọc không ra gì.
                for k in crate::doi_chieu::so(&chu, &nfc) {
                    kq.da_sua.push(SuaDoi::moi(
                        k.cu.clone(),
                        chu[k.cu.clone()].to_string(),
                        k.moi,
                        Loai::Unicode,
                        DoChac::Chac,
                        "dựng lại tổ hợp Unicode (NFC)",
                    ));
                }
                chu = nfc;
            }
        }

        // Tầng 2 — dấu câu, rồi mới tới khoảng trắng.
        let mut s = chuan_hoa::soat_dau_cau(&chu, &tc.chuan_hoa);
        chu = ghi_nhan(&chu, &mut s, &mut kq);
        let mut s = chuan_hoa::soat_khoang_trang(&chu, &tc.chuan_hoa);
        chu = ghi_nhan(&chu, &mut s, &mut kq);

        // Tầng 3 — dấu thanh.
        let mut s = dau_thanh::soat(&chu, self.kieu, tc.nhat_quan_dau_thanh);
        chu = ghi_nhan(&chu, &mut s, &mut kq);

        // Tầng 4 — cặp dễ nhầm. Phần chắc thì áp, phần tuỳ nghĩa để dành.
        if tc.de_nham {
            let tat_ca = self.bang_de_nham.soat(&chu);
            let (mut chac, mo_ho): (Vec<_>, Vec<_>) =
                tat_ca.into_iter().partition(|s| s.do_chac == DoChac::Chac);
            for s in mo_ho {
                kq.cho_xet.push(ChoXet {
                    pham_vi: s.pham_vi.clone(),
                    goc: s.goc.clone(),
                    ung_vien: vec![s.thay_bang.clone()],
                    loai: s.loai,
                    ly_do: s.ly_do.clone(),
                    chac_nho_tu_ghep: false,
                });
            }
            // Vị trí của phần để dành tính trên `chu` trước khi áp phần chắc.
            // Áp xong là lệch hết, nên phải dời lại.
            let truoc = chu.clone();
            chu = ghi_nhan(&chu, &mut chac, &mut kq);
            doi_vi_tri(&truoc, &chu, &mut kq.cho_xet);
        }

        // Tầng 5 — tiếng sai cấu tạo.
        if tc.am_tiet_sai {
            kq.cho_xet.extend(self.soat_am_tiet(&chu));
        }

        kq.chu = chu;
        kq
    }

    /// Tìm những tiếng vừa **không có trong từ điển** vừa **sai cấu tạo**.
    ///
    /// Phải trượt cả hai phép kiểm mới bị bắt, và thứ tự ấy quan trọng. Từ điển
    /// đi trước vì nó phủ được thứ mà bảng vần không bao giờ phủ nổi: từ mượn
    /// viết theo âm Việt (`bêtông`, `micrô`, `pittông`, `rađa`), tên riêng, từ
    /// địa phương. Bảng vần đi sau để đỡ cho những gì từ điển thiếu.
    ///
    /// Chỉ dùng một trong hai là hỏng theo hai kiểu khác nhau: chỉ bảng vần thì
    /// 1.800 từ mượn bị sửa hỏng; chỉ từ điển thì mọi chữ lạ hợp lệ đều bị bắt.
    fn soat_am_tiet(&self, chu: &str) -> Vec<ChoXet> {
        let tu = tach_tu::cat(chu);
        let mut ra = Vec::new();
        for (i, t) in tu.iter().enumerate() {
            let dang_kiem = match tach_tu::dang_tu(t.chu) {
                DangTu::TiengViet => true,
                DangTu::KhongDau => self.tuy_chon.chu_khong_dau,
                _ => false,
            };
            if !dang_kiem || tu_dien::co_am_tiet(t.chu) || am_tiet::hop_le(t.chu) {
                continue;
            }
            // Tên riêng đếm được từ chính cuốn sách. `Kông` trong `Hồng Kông`
            // không có trong từ điển và phạm luật chính tả (`k` không đứng
            // trước `ô`), nhưng nó là tên phiên âm chứ không phải lỗi.
            if self.ten_rieng.contains(&t.chu.to_lowercase()) {
                continue;
            }
            let truoc = i.checked_sub(1).map(|k| tu[k].chu);
            let sau = tu.get(i + 1).map(|x| x.chu);

            let uv_tho = ung_vien::sinh(t.chu);

            // Tách chữ dính xét **trước** phép sửa chữ, vì nó giữ nguyên từng ký
            // tự người ta đã gõ còn sửa chữ thì đoán họ định gõ gì — bằng chứng
            // khác hẳn về chất, không đem ra so bằng cùng một thước được.
            //
            // Không cần chặn thêm ở đây: [`tu_dien::tach_dinh`] đã đòi mảnh sau
            // mở đầu bằng phụ âm, và chính luật ấy loại hết những ca lẽ ra phải
            // sửa như một tiếng (`phảii`, `Huoàng`, `khuyếch`). Từng thử chặn
            // bằng "sửa được thành một tiếng thì đừng tách", nhưng luật ấy loại
            // nhầm cả ca đúng: `Phúlần` xoá chữ `l` ra `Phuần`, một tiếng hợp lệ.
            let tach_duoc: Vec<String> =
                tu_dien::tach_dinh(t.chu).iter().map(|x| chen_khoang_trang(t.chu, x)).collect();

            // Cách tách **dứt khoát** thì áp ngay, không hỏi ai. Cần cả hai vế:
            // cách chia mạnh (mảnh sau mở đầu bằng phụ âm), và hoặc là duy nhất
            // hoặc là dựng lại nguyên một từ ghép có thật (`erằng` → `e rằng`).
            let chac = tach_duoc.first().is_some_and(|top| {
                let thap = top.to_lowercase();
                tu_dien::tach_manh(&thap) && (tach_duoc.len() == 1 || tu_dien::co_tu_ghep(&thap))
            });
            if chac {
                ra.push(ChoXet {
                    pham_vi: t.dau..t.cuoi,
                    goc: t.chu.to_string(),
                    ly_do: format!("`{}` là hai tiếng dính liền — `{}`", t.chu, tach_duoc[0]),
                    ung_vien: tach_duoc,
                    loai: Loai::DinhChu,
                    chac_nho_tu_ghep: true,
                });
                continue;
            }

            // Chưa dứt khoát thì xếp **cách tách và cách sửa chữ chung một
            // bảng**, rồi để bằng chứng từ ghép phân định trên cả hai loại.
            //
            // Xếp hai bảng rời rồi nối lại là sai, và đã sai thật: hễ có một
            // cách tách cạnh tranh là bằng chứng từ ghép bị tắt, nên `Huoàng`
            // (có cách tách vô hại `hu oàng`) không được dùng tới `hoàng tử`
            // trong từ điển, và mô hình tự quyết ra `Hoang` — mất dấu thanh.
            let co_tach = !tach_duoc.is_empty();
            let so_dau = t
                .chu
                .chars()
                .filter(|&c| am_tiet::bo_thanh(c).1 != am_tiet::NGANG)
                .count();
            let (ung_vien, dut_khoat) =
                xep_hang_ung_vien(uv_tho, tach_duoc, so_dau, truoc, sau);
            if ung_vien.is_empty() {
                continue;
            }
            let ly_do = if dut_khoat {
                format!(
                    "`{}` không có trong từ điển; `{}` ghép với chữ bên cạnh thành từ có thật",
                    t.chu, ung_vien[0]
                )
            } else if co_tach {
                format!("`{}` có thể là chữ dính, cũng có thể là lỗi gõ", t.chu)
            } else {
                format!("`{}` không có trong từ điển và sai cấu tạo", t.chu)
            };
            ra.push(ChoXet {
                pham_vi: t.dau..t.cuoi,
                goc: t.chu.to_string(),
                loai: nhan_theo(Loai::AmTietSai, &ung_vien[0]),
                ung_vien,
                ly_do,
                chac_nho_tu_ghep: dut_khoat,
            });
        }
        ra
    }

    /// Nhờ mô hình chọn giữa bản gốc và các ứng viên, rồi áp cái thắng.
    ///
    /// Bản gốc **luôn được chấm cùng** và được cộng thêm ngưỡng — nói cách khác
    /// bản gốc thắng khi hoà. Đây là chỗ chặn cuối: mô hình nhỏ chấm hai câu
    /// gần nhau thường lệch nhau chút đỉnh theo hướng ngẫu nhiên, và mỗi lần
    /// thua sít sao mà vẫn đổi là một chữ của tác giả bị thay không lý do.
    /// `ghi` nhận một dòng mô tả cho **mỗi** chỗ mô hình xét — cả chỗ nó chọn
    /// đổi lẫn chỗ nó để nguyên, kèm chênh lệch điểm. Chỗ để nguyên quan trọng
    /// ngang chỗ đổi: nhìn vào đấy mới biết ngưỡng đang đặt quá chặt hay quá
    /// lỏng, mà không nhìn được thì cái ngưỡng ấy chỉ là một con số bịa.
    pub fn quyet_bang_mo_hinh(
        &self,
        kq: &mut KetQua,
        mo_hinh: &dyn ChamDiem,
        ghi: &mut dyn FnMut(bool, String),
    ) {
        if kq.cho_xet.is_empty() {
            return;
        }
        let goc = kq.chu.clone();
        let mut chon: Vec<SuaDoi> = Vec::new();

        for cx in kq.cho_xet.iter() {
            // Bằng chứng từ ghép **thắng điểm số**, và bỏ qua luôn lượt chấm.
            // Từ điển nói `chúng ta` là một từ còn `chừ ta` thì không — đó là
            // sự thật về tiếng Việt, không phải một ước lượng. Hỏi mô hình ở
            // đây chỉ tạo cơ hội cho nó phủ quyết sai, mà đo được là nó chọn
            // sai khoảng 40% số ca thuộc loại này.
            if cx.chac_nho_tu_ghep {
                ghi(true, format!("`{}` → `{}` (từ ghép trong từ điển)", cx.goc, cx.ung_vien[0]));
                chon.push(SuaDoi::moi(
                    cx.pham_vi.clone(),
                    cx.goc.clone(),
                    cx.ung_vien[0].clone(),
                    cx.loai,
                    DoChac::KhaChac,
                    cx.ly_do.clone(),
                ));
                continue;
            }
            // Chấm trên **câu chứa chỗ sửa**, không phải cả đoạn. Hai lý do:
            // mỗi ứng viên là một lượt chạy mô hình nên đoạn dài đắt gấp bội,
            // và điểm trung bình mỗi token của một đoạn dài bị phần không đổi
            // pha loãng đến mức hai ứng viên gần như bằng nhau — đúng cái ta
            // cần phân biệt thì lại bị làm mờ.
            let cua_so = cau_chua(&goc, &cx.pham_vi);
            let trong_cua_so = cx.pham_vi.start - cua_so.start..cx.pham_vi.end - cua_so.start;
            let nen = &goc[cua_so.clone()];
            let diem_goc = mo_hinh.cham(nen);

            let mut tot: Option<(f32, &String)> = None;
            let mut diem_dau_bang = f32::NEG_INFINITY;
            for (i, uv) in cx.ung_vien.iter().enumerate() {
                let thu = thay_mot_cho(nen, &trong_cua_so, uv);
                let d = mo_hinh.cham(&thu);
                if i == 0 {
                    diem_dau_bang = d;
                }
                if tot.is_none_or(|(dt, _)| d > dt) {
                    tot = Some((d, uv));
                }
            }
            let Some((mut diem, mut uv)) = tot else { continue };

            // **Luật ngôn ngữ là mặc định; mô hình chỉ được lật ngược khi hơn
            // rõ.** Ứng viên đầu bảng do tầng luật xếp — cấu tạo âm tiết, giá
            // phép sửa, bằng chứng từ ghép — và nó chỉ nhường khi mô hình chấm
            // một ứng viên khác hơn hẳn nó, chứ không phải hơn bản gốc.
            //
            // So với bản gốc là so với một chuỗi vô nghĩa, nên ứng viên nào
            // cũng thắng và việc chọn rơi hết vào chênh lệch nhỏ giữa các ứng
            // viên — tức là vào nhiễu của mô hình. Đo được: `nòoài` xếp `ngoài`
            // đầu bảng mà mô hình chọn `ngoai`, mất dấu thanh.
            if !std::ptr::eq(uv, &cx.ung_vien[0])
                && diem - diem_dau_bang <= self.tuy_chon.nguong_mo_hinh
            {
                ghi(
                    false,
                    format!(
                        "giữ thứ tự của luật: `{}` (mô hình nghiêng về `{uv}`, chỉ hơn {:+.3})",
                        cx.ung_vien[0],
                        diem - diem_dau_bang
                    ),
                );
                uv = &cx.ung_vien[0];
                diem = diem_dau_bang;
            }
            let hon = diem - diem_goc;
            if hon <= self.tuy_chon.nguong_mo_hinh {
                ghi(
                    false,
                    format!(
                        "giữ `{}` — ứng viên khá nhất `{uv}` chỉ hơn {hon:+.3}, dưới ngưỡng {:.2}",
                        cx.goc, self.tuy_chon.nguong_mo_hinh
                    ),
                );
                continue;
            }
            ghi(true, format!("`{}` → `{uv}` (hơn {hon:+.3})", cx.goc));
            chon.push(SuaDoi::moi(
                cx.pham_vi.clone(),
                cx.goc.clone(),
                uv.clone(),
                // Nhãn theo **cách sửa được chọn**, không theo phỏng đoán lúc
                // dò: một chỗ ngờ có thể mang cả ứng viên tách chữ lẫn ứng viên
                // sửa chữ, và người đọc báo cáo cần biết cái nào đã thắng.
                nhan_theo(cx.loai, uv),
                DoChac::NgoVuc,
                format!("{} — mô hình chấm hơn {hon:+.2}", cx.ly_do),
            ));
        }
        kq.cho_xet.clear();
        let moi = ghi_nhan(&kq.chu.clone(), &mut chon, kq);
        kq.chu = moi;
    }

    /// Khi không có mô hình: chỉ áp những chỗ có **đúng một** ứng viên.
    ///
    /// Nhiều hơn một ứng viên nghĩa là bộ dò không phân được, mà không phân
    /// được thì đoán bừa. Để nguyên và ghi vào báo cáo phần "chưa sửa".
    pub fn quyet_khong_mo_hinh(&self, kq: &mut KetQua) {
        let mut chon: Vec<SuaDoi> = Vec::new();
        let mut con_lai = Vec::new();
        for cx in std::mem::take(&mut kq.cho_xet) {
            if cx.chac_nho_tu_ghep {
                chon.push(SuaDoi::moi(
                    cx.pham_vi.clone(),
                    cx.goc.clone(),
                    cx.ung_vien[0].clone(),
                    cx.loai,
                    DoChac::KhaChac,
                    cx.ly_do.clone(),
                ));
                continue;
            }
            if cx.ung_vien.len() == 1 && cx.loai == Loai::AmTietSai {
                chon.push(SuaDoi::moi(
                    cx.pham_vi.clone(),
                    cx.goc.clone(),
                    cx.ung_vien[0].clone(),
                    cx.loai,
                    DoChac::KhaChac,
                    format!("{} — chỉ có một cách sửa", cx.ly_do),
                ));
            } else {
                con_lai.push(cx);
            }
        }
        let moi = ghi_nhan(&kq.chu.clone(), &mut chon, kq);
        kq.chu = moi;
        kq.cho_xet = con_lai;
    }
}

/// Khoảng của câu chứa `r`.
///
/// Ranh giới câu nhận bằng `.`, `!`, `?`, `…` có khoảng trắng theo sau — cách
/// nhận thô nhưng ở đây không cần chính xác, chỉ cần **ổn định**: cùng một chỗ
/// sửa thì bản gốc và mọi ứng viên phải được cắt ra cùng một cửa sổ, không thì
/// đem so hai điểm của hai câu dài ngắn khác nhau.
///
/// Có chặn trên: câu dài quá thì cắt còn khoảng 400 byte quanh chỗ sửa, vì
/// sách dịch có những "câu" cả nghìn chữ không một dấu chấm.
fn cau_chua(chu: &str, r: &Range<usize>) -> Range<usize> {
    const TOI_DA: usize = 400;
    let mut dau = 0usize;
    let b = chu.as_bytes();
    for i in (0..r.start).rev() {
        if matches!(b[i], b'.' | b'!' | b'?') && b.get(i + 1) == Some(&b' ') {
            dau = i + 2;
            break;
        }
    }
    let mut cuoi = chu.len();
    for i in r.end..b.len() {
        if matches!(b[i], b'.' | b'!' | b'?') {
            cuoi = (i + 1).min(chu.len());
            break;
        }
    }
    if cuoi - dau > TOI_DA {
        dau = dau.max(r.start.saturating_sub(TOI_DA / 2));
        cuoi = cuoi.min(r.end + TOI_DA / 2);
        while !chu.is_char_boundary(dau) {
            dau += 1;
        }
        while !chu.is_char_boundary(cuoi) {
            cuoi -= 1;
        }
    }
    dau..cuoi
}

/// Sinh ứng viên rồi xếp lại theo bằng chứng **từ ghép**.
///
/// Phép sinh xếp hạng theo "khác bản gốc ít nhất", tức là theo hình dạng lỗi
/// gõ. Nhưng hình dạng không phân được những ứng viên cùng giá, mà chuyện ấy
/// xảy ra liên tục: `chúg` cho ra `chúng`, `chừ`, `chú`, `chug`… đều cách bản
/// gốc một bước. Ở đó thì hàng xóm quyết định — `chúng ta` có trong từ điển,
/// `chừ ta` thì không.
///
/// Đo trên một cuốn sách: mô hình ngôn ngữ 9 tỷ tham số chọn sai khoảng 40% số
/// ca thuộc loại này, và phần lớn ca sai đều là ca mà từ ghép phân được ngay.
/// Nên bằng chứng từ ghép đặt **trước** mô hình, không phải sau.
fn xep_hang_ung_vien(
    mut uv: Vec<ung_vien::UngVien>,
    tach: Vec<String>,
    so_dau_thanh: usize,
    truoc: Option<&str>,
    sau: Option<&str>,
) -> (Vec<String>, bool) {
    // Giá của một cách tách.
    //
    // **Rẻ nhất** khi nó đáng tin: nó không đổi một ký tự nào, chỉ thêm khoảng
    // trắng, nên gần bản gốc hơn mọi phép sửa chữ (phép rẻ nhất trong số ấy là
    // đảo hai chữ, giá 4).
    const GIA_TACH: u32 = 3;
    // **Đắt hơn mọi phép sửa chữ** khi nó đáng ngờ. Cách tách yếu — mảnh sau mở
    // đầu bằng nguyên âm — mà đem cho giá rẻ thì `phảii` cho ra `phả ii` đứng
    // trên `phải`, vì từ điển có cả mục `ii`.
    const GIA_TACH_YEU: u32 = 9;

    uv.extend(tach.into_iter().map(|chu| {
        let thap = chu.to_lowercase();
        // Cách tách yếu vẫn được coi là đáng tin khi chuỗi gốc mang **từ hai
        // dấu thanh trở lên**: một âm tiết tiếng Việt chỉ mang được một dấu
        // thanh, nên chuỗi ấy chắc chắn không phải một tiếng và phải tách.
        //
        // Đây là chỗ phân `ngồiở` (hai dấu → `ngồi ở`) với `phảii` (một dấu →
        // `phải`). Cả hai đều có mảnh sau mở đầu bằng nguyên âm.
        let dang_tin = tu_dien::tach_manh(&thap) || so_dau_thanh > 1;
        let gia = if dang_tin { GIA_TACH } else { GIA_TACH_YEU };
        ung_vien::UngVien { chu, gia }
    }));
    if uv.is_empty() {
        return (Vec::new(), false);
    }
    // Ứng viên không có trong từ điển thì bỏ hẳn: sửa một chữ không tồn tại
    // thành một chữ khác cũng không tồn tại là đổi lỗi này lấy lỗi kia. Nhưng
    // chỉ bỏ khi còn lại thứ gì đó — từ điển không phủ hết tên riêng.
    let co_trong_tu_dien = uv.iter().any(|u| tu_dien::ung_vien_co_that(&u.chu));
    if co_trong_tu_dien {
        uv.retain(|u| tu_dien::ung_vien_co_that(&u.chu));
    }
    let khop: Vec<usize> =
        uv.iter().map(|u| tu_dien::khop_hang_xom(truoc, &u.chu, sau)).collect();
    let mut ghep: Vec<(usize, ung_vien::UngVien)> = khop.into_iter().zip(uv).collect();
    // Khớp hàng xóm là tiêu chí đầu; giá của phép sinh phân định trong nội bộ
    // từng mức khớp.
    ghep.sort_by_key(|(k, u)| (std::cmp::Reverse(*k), u.gia));

    // Bằng chứng dứt khoát khi ứng viên đầu bảng **thắng không hoà**: nó ghép
    // được với hàng xóm, và không ứng viên nào khác cùng mức khớp mà giá bằng.
    //
    // Tiêu chí đầu tiên là "đúng một ứng viên ghép được", nhưng nó hỏng ngay khi
    // phép sinh mạnh lên: `tình thuơng` có tới mấy ứng viên ghép được với
    // `tình`, mà `thương` vẫn hơn hẳn phần còn lại vì nó chỉ khác bản gốc ở dấu.
    // Đòi độc nhất thì mất luôn ca dễ nhất.
    let dut_khoat = ghep.first().is_some_and(|(k, u)| {
        *k > 0 && ghep.get(1).is_none_or(|(k2, u2)| (*k, u.gia) != (*k2, u2.gia))
    });
    (ghep.into_iter().map(|(_, u)| u.chu).collect(), dut_khoat)
}

/// Số lần một chữ phải xuất hiện thì mới được coi là tên riêng.
///
/// Ba là chỗ đo được: lỗi gõ lặp lại y hệt ba lần trong một cuốn sách là hiếm,
/// còn tên riêng thì gặp hàng chục lần.
const LAN_DE_LA_TEN_RIENG: usize = 3;

/// Gom tên riêng từ chính cuốn sách: chữ **viết hoa**, **không có trong từ
/// điển**, và **lặp lại nhiều lần**.
///
/// Gọi cho mọi đoạn ở lượt đọc đầu, rồi đưa kết quả vào [`BoSoat::voi_ten_rieng`].
///
/// Vì sao phải đếm cả sách thay vì nhìn một chữ: `Kông` trong `Hồng Kông` và
/// `Kó` (gõ nhầm `Có`) trông giống hệt nhau nếu chỉ nhìn một chữ — cùng viết
/// hoa, cùng không có trong từ điển, cùng phạm luật `k` không đứng trước
/// nguyên âm sau. Chỉ có số lần xuất hiện phân được: tên riêng lặp lại, lỗi gõ
/// thì không.
///
/// Đòi **viết hoa** là điều kiện then chốt. Không có nó thì `khôgn` — gõ nhầm
/// `không`, lặp 153 lần trong một cuốn — cũng thành "tên riêng" và thoát hết.
pub fn gom_ten_rieng(van_ban: &str, dem: &mut std::collections::HashMap<String, usize>) {
    let tu = tach_tu::cat(van_ban);
    for (i, t) in tu.iter().enumerate() {
        if tach_tu::dang_tu(t.chu) != DangTu::TiengViet {
            continue;
        }
        if !t.chu.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }
        if tu_dien::co_am_tiet(t.chu) {
            continue;
        }
        // **Chữ hoa phải nằm giữa câu**, không phải đầu câu. Đầu câu thì chữ
        // nào cũng viết hoa nên chữ hoa chẳng nói lên điều gì — mà lỗi gõ hay
        // rơi vào đầu câu đúng như mọi chỗ khác. Không có luật này thì `Chẵng`
        // (gõ nhầm `Chẳng`, lặp mấy chục lần) được xếp là tên riêng rồi thoát.
        //
        // Giữa câu thì tiếng Việt chỉ viết hoa cho tên riêng, nên tín hiệu rất
        // mạnh — đủ mạnh để giữ `Kông` trong `Hồng Kông`.
        if i == 0 || !giua_cau(van_ban, tu[i - 1].cuoi, t.dau) {
            continue;
        }
        *dem.entry(t.chu.to_lowercase()).or_insert(0) += 1;
    }
}

/// Khoảng giữa hai chữ có phải chỉ là khoảng trắng thường không.
///
/// Có dấu kết câu, dấu ngoặc kép hay xuống dòng xen vào thì chữ sau là **đầu
/// câu**, và việc nó viết hoa không nói lên điều gì.
fn giua_cau(van_ban: &str, tu_vi_tri: usize, den: usize) -> bool {
    van_ban[tu_vi_tri..den].chars().all(|c| c == ' ' || c == ',' || c == ';')
}

/// Chốt danh sách tên riêng từ bảng đếm.
pub fn chot_ten_rieng(
    dem: std::collections::HashMap<String, usize>,
) -> std::collections::HashSet<String> {
    dem.into_iter().filter(|(_, n)| *n >= LAN_DE_LA_TEN_RIENG).map(|(t, _)| t).collect()
}

/// Chèn khoảng trắng vào **chính chuỗi gốc**, theo cách chia đã tìm được.
///
/// [`tu_dien::tach_dinh`] làm việc trên bản viết thường nên kết quả của nó mất
/// hết chữ hoa. Không được lấy thẳng: chữ dính hay dính đúng ở chỗ tên riêng
/// (`mựcMinh`, `HuyềnVũ`, `LãoTạ`), mà đó cũng là chỗ chữ hoa mang thông tin.
/// Lấy thẳng thì ra `mực minh`, `Huyền vũ` — tách đúng chỗ nhưng xoá mất tên
/// người.
///
/// Nên chỉ lấy **vị trí ngắt** từ cách chia, rồi cắt trên chuỗi gốc. Như thế
/// từng ký tự giữ nguyên hình dạng người ta đã gõ, và phép sửa này thật sự chỉ
/// thêm vào mấy khoảng trắng.
fn chen_khoang_trang(goc: &str, cach_chia: &str) -> String {
    let ky_tu: Vec<char> = goc.chars().collect();
    let mut ra = String::with_capacity(goc.len() + 4);
    let mut vt = 0usize;
    for (i, manh) in cach_chia.split(' ').enumerate() {
        let n = manh.chars().count();
        if vt + n > ky_tu.len() {
            // Cách chia không khớp độ dài chuỗi gốc — không thể xảy ra, nhưng
            // thà trả về bản viết thường còn hơn cắt vào giữa chữ.
            return cach_chia.to_string();
        }
        if i > 0 {
            ra.push(' ');
        }
        ra.extend(&ky_tu[vt..vt + n]);
        vt += n;
    }
    if vt != ky_tu.len() {
        return cach_chia.to_string();
    }
    ra
}

/// Nhãn loại lỗi suy từ cách sửa đã chọn: có khoảng trắng nghĩa là đã tách chữ.
fn nhan_theo(loai: Loai, da_chon: &str) -> Loai {
    if !matches!(loai, Loai::DinhChu | Loai::AmTietSai) {
        return loai;
    }
    if da_chon.contains(' ') {
        Loai::DinhChu
    } else {
        Loai::AmTietSai
    }
}

fn thay_mot_cho(goc: &str, r: &Range<usize>, moi: &str) -> String {
    let mut s = String::with_capacity(goc.len() + moi.len());
    s.push_str(&goc[..r.start]);
    s.push_str(moi);
    s.push_str(&goc[r.end..]);
    s
}

/// Áp một loạt phép sửa, ghi những phép **thật sự áp được** vào kết quả.
///
/// Không ghi thẳng cả danh sách vào báo cáo: `ap_dung` bỏ phép chồng nhau, mà
/// báo cáo liệt kê một phép sửa không xảy ra thì người dùng đi tìm nó trong
/// sách và không thấy.
fn ghi_nhan(chu: &str, s: &mut Vec<SuaDoi>, kq: &mut KetQua) -> String {
    s.retain(|x| x.co_doi());
    if s.is_empty() {
        return chu.to_string();
    }
    let (moi, _) = ap_dung(chu, s);
    // `ap_dung` đã sắp `s` và bỏ phép chồng; dựng lại danh sách áp được bằng
    // cùng luật để báo cáo khớp đúng những gì đã xảy ra.
    let mut cuoi = 0usize;
    for x in s.iter() {
        if x.pham_vi.start < cuoi || x.pham_vi.end > chu.len() {
            continue;
        }
        cuoi = x.pham_vi.end;
        kq.da_sua.push(x.clone());
    }
    moi
}

/// Dời vị trí các chỗ-để-xét sau khi chuỗi đã đổi.
///
/// Cách làm thô nhưng đúng: tìm lại chuỗi gốc quanh vị trí cũ trong chuỗi mới.
/// Không tìm thấy thì bỏ chỗ ấy — thà bỏ sót còn hơn sửa nhầm vị trí, vì sửa
/// nhầm vị trí là cắt vào giữa một chữ khác.
fn doi_vi_tri(truoc: &str, sau: &str, cho_xet: &mut Vec<ChoXet>) {
    if truoc == sau {
        return;
    }
    cho_xet.retain_mut(|cx| {
        // Neo bằng chính chuỗi gốc, tìm chỗ gần vị trí cũ nhất.
        let mut tot: Option<usize> = None;
        let mut tu = 0usize;
        while let Some(k) = sau[tu..].find(&cx.goc) {
            let vt = tu + k;
            if tot.is_none_or(|t| {
                vt.abs_diff(cx.pham_vi.start) < t.abs_diff(cx.pham_vi.start)
            }) {
                tot = Some(vt);
            }
            tu = vt + 1;
            if tu >= sau.len() {
                break;
            }
        }
        match tot {
            Some(vt) => {
                cx.pham_vi = vt..vt + cx.goc.len();
                true
            }
            None => false,
        }
    });
}

#[cfg(test)]
mod kiem {
    use super::*;

    /// Mô hình giả: chấm điểm bằng cách đếm cụm từ trong danh sách "câu tự
    /// nhiên". Đủ để kiểm phần điều phối mà không cần nạp mô hình thật.
    struct MoHinhGia(Vec<&'static str>);
    impl ChamDiem for MoHinhGia {
        fn cham(&self, cau: &str) -> f32 {
            let thap = cau.to_lowercase();
            self.0.iter().filter(|m| thap.contains(*m)).count() as f32
        }
    }

    fn bo() -> BoSoat {
        BoSoat::moi(TuyChon::default(), Kieu::Cu)
    }

    #[test]
    fn chay_het_cac_tang_tren_mot_doan() {
        let mut kq = bo().soat("Anh ấy xử dụng máy tính , rồi   đi ra .");
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, "Anh ấy sử dụng máy tính, rồi đi ra.");
        assert!(kq.da_sua.len() >= 3, "{:?}", kq.da_sua);
    }

    #[test]
    fn van_ban_sach_thi_khong_doi_gi() {
        let v = "Anh ấy nói: “Tôi không biết.” Rồi bỏ đi, chẳng ngoái lại.";
        let mut kq = bo().soat(v);
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
        assert!(kq.da_sua.is_empty(), "{:?}", kq.da_sua);
        assert!(kq.cho_xet.is_empty(), "{:?}", kq.cho_xet);
    }

    #[test]
    fn tieng_sai_nhieu_ung_vien_thi_de_lai_cho_mo_hinh() {
        let kq = bo().soat("Tình thuơng của mẹ");
        assert_eq!(kq.cho_xet.len(), 1);
        assert!(kq.cho_xet[0].ung_vien.contains(&"thương".to_string()));
    }

    #[test]
    fn mo_hinh_chon_ung_vien_dung() {
        let mh = MoHinhGia(vec!["tình thương"]);
        let mut kq = bo().soat("Tình thuơng của mẹ");
        bo().quyet_bang_mo_hinh(&mut kq, &mh, &mut |_, _| {});
        assert_eq!(kq.chu, "Tình thương của mẹ");
    }

    struct Deu;
    impl ChamDiem for Deu {
        fn cham(&self, _: &str) -> f32 {
            1.0
        }
    }

    #[test]
    fn mo_hinh_thang_sit_sao_thi_khong_doi() {
        // Mô hình chấm mọi ứng viên bằng nhau — không có bằng chứng gì. Bản gốc
        // phải sống sót. Đây là cái van chặn ứng dụng tự sửa bừa.
        //
        // Câu phải chọn sao cho **không ứng viên nào ghép được với hàng xóm**,
        // không thì bằng chứng từ ghép quyết trước và mô hình chẳng được hỏi.
        let v = "Ừ thuơng à";
        let mut kq = bo().soat(v);
        assert!(!kq.cho_xet[0].chac_nho_tu_ghep, "ca này lẽ ra không có bằng chứng từ ghép");
        bo().quyet_bang_mo_hinh(&mut kq, &Deu, &mut |_, _| {});
        assert_eq!(kq.chu, v);
    }

    #[test]
    fn tu_ghep_quyet_truoc_mo_hinh() {
        // `tình thương` có trong từ điển, `tình thường`/`tình thưởng` thì không.
        // Bằng chứng ấy dứt khoát nên phải áp được **kể cả khi mô hình phản
        // đối** — mô hình chấm đều nhau ở đây, tức là nó không có ý kiến gì.
        let mut kq = bo().soat("Tình thuơng của mẹ");
        assert!(kq.cho_xet[0].chac_nho_tu_ghep);
        bo().quyet_bang_mo_hinh(&mut kq, &Deu, &mut |_, _| {});
        assert_eq!(kq.chu, "Tình thương của mẹ");
    }

    #[test]
    fn tu_ghep_sua_duoc_khi_khong_co_mo_hinh() {
        // Không có card đồ hoạ thì vẫn sửa được lớp lỗi này.
        let mut kq = bo().soat("Tình thuơng của mẹ");
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, "Tình thương của mẹ");
    }

    #[test]
    fn ten_rieng_lap_lai_thi_duoc_giu_nguyen() {
        // `Kông` không có trong từ điển và phạm luật chính tả (`k` không đứng
        // trước `ô`), nhưng trong `Hồng Kông` nó là tên phiên âm chứ không phải
        // lỗi. Phân được nhờ đếm cả sách: tên riêng lặp lại, lỗi gõ thì không.
        let mut dem = std::collections::HashMap::new();
        for cau in [
            "Tôi đến Hồng Kông chơi.",
            "Ở Hồng Kông có nhiều người.",
            "Chuyến bay tới Hồng Kông bị hoãn.",
        ] {
            gom_ten_rieng(cau, &mut dem);
        }
        let ten = chot_ten_rieng(dem);
        assert!(ten.contains("kông"), "{ten:?}");

        let bo = BoSoat::moi(TuyChon::default(), Kieu::Cu).voi_ten_rieng(ten);
        let v = "Tôi đến Hồng Kông chơi.";
        let mut kq = bo.soat(v);
        bo.quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
    }

    #[test]
    fn chu_hoa_dau_cau_khong_phai_ten_rieng() {
        // `Chẵng` là gõ nhầm `Chẳng`, và nó đứng đầu câu nên chữ hoa chẳng nói
        // lên điều gì. Không có luật này thì mọi lỗi gõ hay rơi vào đầu câu đều
        // được xếp là tên riêng rồi thoát hết.
        let mut dem = std::collections::HashMap::new();
        for _ in 0..5 {
            gom_ten_rieng("Chẵng ai biết. Chẵng ai hay.", &mut dem);
        }
        assert!(chot_ten_rieng(dem).is_empty());
    }

    #[test]
    fn d_va_d_gach_deu_la_tu_that_nen_khong_ai_dung_vao() {
        // `dang` (dang tay, dở dang) và `đang` (đang làm) là hai từ khác nhau,
        // cả hai đều có trong từ điển. Bộ dò chỉ xét chữ **không có** trong từ
        // điển, nên không chữ nào trong hai chữ này từng bị đem ra cân nhắc —
        // dù phép sinh ứng viên có biết đường `dang` → `đang` đi nữa.
        for v in ["Tay dang rộng, việc còn dở dang.", "Anh đang làm gì đấy?"] {
            let mut kq = bo().soat(v);
            bo().quyet_khong_mo_hinh(&mut kq);
            assert_eq!(kq.chu, v);
            assert!(kq.cho_xet.is_empty(), "{:?}", kq.cho_xet);
        }
    }

    #[test]
    fn khong_dung_vao_tu_muon_co_trong_tu_dien() {
        // `bêtông`, `micrô`, `rađa` sai cấu tạo âm tiết nhưng có trong từ điển.
        // Trước khi có tầng từ điển, cả 1.800 chữ kiểu này bị sửa hỏng.
        let v = "Cột bêtông và cái micrô cùng cái rađa";
        let mut kq = bo().soat(v);
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
        assert!(kq.cho_xet.is_empty(), "{:?}", kq.cho_xet);
    }

    #[test]
    fn khong_dung_vao_ten_rieng_nuoc_ngoai() {
        // `Dumbledore`, `Voldemort` không phải tiếng Việt và cũng không mang
        // dấu — mặc định không đụng tới.
        let v = "Giáo sư Dumbledore nhìn Voldemort.";
        let mut kq = bo().soat(v);
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
    }
}

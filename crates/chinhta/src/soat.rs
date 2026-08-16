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
    /// Log-xác suất trung bình mỗi token của câu. **Càng cao càng tự nhiên.**
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
    /// Mô hình phải chấm cách sửa hơn bản gốc bao nhiêu thì mới đổi.
    ///
    /// Đây là **cái van an toàn của cả ứng dụng**. Ứng dụng tự sửa rồi mới báo
    /// cáo, nên với những chỗ mơ hồ, "mô hình thấy hơi khá hơn" là chưa đủ —
    /// hơn sít sao thì phần lớn là nhiễu. Mức 0,15 nats/token đo trên vài mô
    /// hình nhỏ là chỗ mà cách sửa đúng tách hẳn khỏi cách sửa đoán mò.
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
            nguong_mo_hinh: 0.15,
        }
    }
}

pub struct BoSoat {
    pub tuy_chon: TuyChon,
    bang_de_nham: de_nham::Bang,
    kieu: Kieu,
}

impl BoSoat {
    pub fn moi(tuy_chon: TuyChon, kieu: Kieu) -> BoSoat {
        BoSoat { tuy_chon, bang_de_nham: de_nham::Bang::nap(), kieu }
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
            let tach_duoc = tu_dien::tach_dinh(t.chu);
            if !tach_duoc.is_empty() {
                let ung_vien: Vec<String> =
                    tach_duoc.iter().map(|x| chen_khoang_trang(t.chu, x)).collect();
                ra.push(ChoXet {
                    pham_vi: t.dau..t.cuoi,
                    goc: t.chu.to_string(),
                    ly_do: format!("`{}` là hai tiếng dính liền — `{}`", t.chu, ung_vien[0]),
                    ung_vien,
                    loai: Loai::DinhChu,
                    // Dứt khoát khi **chỉ có một** cách chia. Nhiều cách chia
                    // thì phải cân nhắc thật, vì chọn sai chỗ ngắt là đổi hẳn
                    // nghĩa câu.
                    chac_nho_tu_ghep: tach_duoc.len() == 1,
                });
                continue;
            }

            let (uv, dut_khoat) = xep_hang_ung_vien(uv_tho, truoc, sau);
            if uv.is_empty() {
                continue;
            }
            let ly_do = if dut_khoat {
                format!("`{}` không có trong từ điển; `{}` ghép với chữ bên cạnh thành từ có thật", t.chu, uv[0])
            } else {
                format!("`{}` không có trong từ điển và sai cấu tạo", t.chu)
            };
            ra.push(ChoXet {
                pham_vi: t.dau..t.cuoi,
                goc: t.chu.to_string(),
                ung_vien: uv,
                loai: Loai::AmTietSai,
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
            for uv in cx.ung_vien.iter() {
                let thu = thay_mot_cho(nen, &trong_cua_so, uv);
                let d = mo_hinh.cham(&thu);
                if tot.is_none_or(|(dt, _)| d > dt) {
                    tot = Some((d, uv));
                }
            }
            let Some((diem, uv)) = tot else { continue };
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
                cx.loai,
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
    truoc: Option<&str>,
    sau: Option<&str>,
) -> (Vec<String>, bool) {
    if uv.is_empty() {
        return (Vec::new(), false);
    }
    // Ứng viên không có trong từ điển thì bỏ hẳn: sửa một chữ không tồn tại
    // thành một chữ khác cũng không tồn tại là đổi lỗi này lấy lỗi kia. Nhưng
    // chỉ bỏ khi còn lại thứ gì đó — từ điển không phủ hết tên riêng.
    let co_trong_tu_dien = uv.iter().any(|u| tu_dien::co_am_tiet(&u.chu));
    if co_trong_tu_dien {
        uv.retain(|u| tu_dien::co_am_tiet(&u.chu));
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

# CLAUDE.md

Hướng dẫn cho Claude Code (claude.ai/code) khi làm việc trong repo này.

## Ngôn ngữ

Toàn bộ mã nguồn, comment, tên biến/hàm, tên file và commit message dùng **tiếng
Việt** (không dấu cho định danh, có dấu trong comment). Giữ nguyên quy ước này —
đừng chen tên tiếng Anh vào giữa.

Comment ở đầu mỗi file giải thích *vì sao* chứ không phải *cái gì*, và thường ghi
kèm **số đo thực nghiệm**. Sửa logic có số đo thì cập nhật luôn con số. Gần như
mọi lưới chặn trong repo này đều suy từ số liệu đo trên sách thật chứ không từ
suy đoán — comment phải nói ra số liệu ấy, không thì người sau tưởng là tuỳ tiện
và bỏ đi.

## Lệnh thường dùng

```bash
cargo build --release
cargo test --release --workspace
cargo run --release -p giaodien --bin vie-spellcheck      # cửa sổ
cargo run --release -p giaodien --bin vsc -- sach.epub --kho   # dòng lệnh, chạy khô
```

**Dùng `--release` cho cả `cargo test`.** Không có cờ ấy thì cargo dựng
llama-cpp-sys-2 ở profile debug, tức là **biên dịch lại toàn bộ nhân CUDA của
llama.cpp** — hàng chục phút, để chạy một bộ kiểm mất hai giây.

### Dựng cần gì

- **Visual Studio Build Tools** (workload C++), **cmake**, và **`LIBCLANG_PATH`**
  trỏ vào thư mục chứa `libclang.dll` — `llama-cpp-sys-2` dựng llama.cpp bằng
  cmake và sinh binding bằng bindgen.
- **CUDA Toolkit ≥ 12.8** (card Blackwell / RTX 50xx cần từ bản này).

Dựng không cần CUDA: `cargo build --release --no-default-features`. Bản ấy chạy
được mọi tầng luật, chỉ không nạp được mô hình.

### Ba cạm bẫy của bản dựng CUDA

**`MSB1009: Project file does not exist`** nghĩa là cmake đã hỏng dở một lần
trước đó và để lại `CMakeCache.txt`; lần sau nó bỏ qua bước cấu hình rồi MSBuild
không có gì để dựng. `cargo clean` **không** gỡ được — phải xoá hẳn:

```bash
rm -rf target/release/build/llama-cpp-sys-2-* target/debug/build/llama-cpp-sys-2-*
```

**Thiếu `CUDA_PATH_V13_3`** thì cmake báo `The CUDA Toolkit directory '' does not
exist`. Bản cài đặt CUDA có đặt biến ấy ở mức máy, nhưng **shell mở trước lúc cài
thì không thấy nó** — mở terminal mới.

**`cuda-arch.cmake` ghim kiến trúc CUDA cần dựng** (`120a-real` = Blackwell,
RTX 50xx). Đổi card thì sửa file ấy. Không có nó thì llama.cpp dựng cho cả bảy
đời card và mất hàng chục phút để sinh ra sáu bản mã máy này không bao giờ chạy.
`.cargo/config.toml` trỏ cmake vào file ấy qua `CMAKE_TOOLCHAIN_FILE`; dùng
đường vòng ấy vì `build.rs` của llama-cpp-sys-2 tự đặt `GGML_NATIVE=OFF` sau khi
đọc biến môi trường nên đặt biến không ăn thua.

**CUDA 13 dời DLL runtime sang `bin\x64`.** `crates/giaodien/build.rs` chép
`cublas64_*.dll`, `cublasLt64_*.dll`, `cudart64_*.dll` sang cạnh file thực thi.
Thiếu chúng thì Windows chặn ngay ở bước nạp ảnh — hộp thoại đỏ "The code
execution cannot proceed", chưa vào được dòng `main` nào nên không có chỗ nào báo
lỗi tử tế hơn. Đây không phải chuyện chỉ thiếu PATH: cho `%CUDA_PATH%\bin` vào
PATH cũng không đủ vì DLL không còn nằm ở đó nữa.

## Kiến trúc

```
crates/chinhta/    lõi ngôn ngữ — không phụ thuộc EPUB hay llama.cpp
  am_tiet.rs       cấu tạo âm tiết: âm đầu + vần + thanh
  tu_dien.rs       kho âm tiết và kho từ ghép; tách chữ dính
  dau_thanh.rs     vị trí dấu thanh, kiểu cũ/mới
  chuan_hoa.rs     Unicode, khoảng trắng, dấu câu
  de_nham.rs       cặp dễ nhầm (bảng ở du-lieu/de-nham.txt)
  ung_vien.rs      từ tiếng sai sinh ra các cách sửa
  doi_chieu.rs     so bản trước/sau, ra khoảng byte cần vá
  soat.rs          điều phối các tầng
crates/sach/       đọc/ghi EPUB giữ nguyên phần không sửa
crates/mohinh/     chấm điểm câu bằng llama.cpp trên GPU
crates/giaodien/   ứng dụng (cửa sổ egui + dòng lệnh), nhật ký, báo cáo
```

### Đường đi của dữ liệu

```
EPUB → sach::quet     tách file XHTML thành đoạn, nối các nút bị thẻ inline cắt
     → chinhta::soat  chạy các tầng, ra (chữ đã sửa, danh sách chỗ ngờ)
     → mohinh         chấm điểm chọn giữa các ứng viên  [tuỳ chọn]
     → doi_chieu      so bản trước/sau → khoảng byte cần vá
     → sach::ghi      vá đúng khoảng ấy, chép nguyên phần còn lại
```

## Những quyết định định hình cả phần còn lại

### Ứng dụng tự sửa rồi mới báo cáo

Nên **sửa nhầm đắt hơn bỏ sót nhiều lần**. Mọi lưới chặn trong repo đều nghiêng
về phía ấy. Khi cân nhắc thêm một luật mới, hỏi: luật này làm hỏng chữ đúng
trong bao nhiêu phần trăm ca? Nếu không đo được thì đừng thêm.

Bản gốc **không bao giờ bị ghi đè** — kết quả luôn ra file khác, và `bat_dau`
chặn cả khi người dùng cố chọn trùng.

### Mô hình ngôn ngữ **không được sinh ra chữ nào**

Nó chỉ chấm điểm câu. Các cách sửa do tầng luật và tầng cấu tạo âm tiết sinh ra —
chúng bảo đảm mọi ứng viên đều là tiếng Việt viết đúng — còn mô hình chỉ **xếp
hạng**. Nhờ thế nó không viết lại được văn của tác giả, không bịa tên riêng, và
kết quả lặp lại được nên kiểm thử được.

Đừng đổi sang kiểu "đưa câu cho mô hình bảo nó sửa". Đã cân nhắc và bác: mô hình
tiện tay đổi từ ngữ, gộp câu, bỏ chữ nó thấy thừa. Trong tiểu thuyết thì đó
không phải sửa lỗi, đó là hỏng sách.

### Ba tầng xác nhận một tiếng, thứ tự không đảo được

1. **Từ điển** (9.550 âm tiết) — có thì thôi.
2. **Cấu tạo âm tiết** — đỡ cho những gì từ điển thiếu.
3. Trượt cả hai mới sinh cách sửa, rồi **từ ghép** (69.893 mục) chọn giữa chúng.

Chỉ dùng bảng vần thì **1.813 từ mượn** viết theo âm Việt (`bêtông`, `micrô`,
`rađa`, `pittông`) bị sửa hỏng — bảng vần mô hình hoá ngữ âm tiếng Việt nên nó
bác bỏ mọi thứ ngoài hệ thống ấy. Chỉ dùng từ điển thì mọi chữ lạ hợp lệ đều bị
bắt, và `híc` không nằm trong bộ từ điển nào cả.

### Bằng chứng từ ghép đặt **trước** mô hình

`chúng ta` có trong từ điển còn `chừ ta` thì không — đó là sự thật về tiếng Việt,
không phải một ước lượng. Đo được: mô hình 9 tỷ tham số chọn sai khoảng **40%**
số ca thuộc loại này, mà từ ghép phân được ngay và không cần card đồ hoạ.

### Tầng luật là mặc định, mô hình chỉ **lật ngược** khi hơn rõ

Ứng viên đầu bảng do tầng luật xếp; mô hình phải chấm một ứng viên khác hơn **nó**
quá ngưỡng thì mới được đổi — chứ không phải hơn bản gốc. So với bản gốc là so
với một chuỗi vô nghĩa nên ứng viên nào cũng thắng, và việc chọn rơi hết vào nhiễu.

## Cạm bẫy

**Dựng lại NFC trước khi cắt từ.** Dấu tổ hợp không phải chữ cái, nên chữ gõ rời
bị cắt đôi ngay giữa từ: `Huyền` ra thành `Huyê` + `n`. Đã vấp **hai lần** — một
lần ở công cụ soi `soi_van.rs` (291 lần `tâ` đứng đầu bảng, che mất lỗ hổng
thật), một lần ở bộ gom tên riêng (bảng đầy mảnh chữ cụt). `soat::soat` đã làm
đúng thứ tự; mọi chỗ **khác** đọc `Doan::chu` đều phải tự dựng lại NFC.

**Chấm điểm chia cho số ký tự, không chia cho số token.** Thêm một token dễ đoán
có thể *nâng* điểm trung bình lên, nên mô hình từng chấm `Hắn phảii đi ngay.` cao
hơn `Hắn phải đi ngay.` — câu sai thắng câu đúng. Ngưỡng tính bằng nats/**ký tự**
nên con số nhỏ hơn hẳn mức quen thuộc (mặc định 0,03).

**`đ` không đối xứng với `d`.** Gõ thiếu dấu (`dang` khi định gõ `đang`) là
chuyện thường; chiều ngược lại thì không, vì muốn ra `đ` phải gõ hẳn `dd`. Xếp
hai chiều như nhau thì bộ sửa đề nghị đổi `đ` thành `d` — hai phụ âm khác nhau.

**Giá của một phép sửa phải phản ánh "giữ được bao nhiêu thứ người ta đã gõ".**
Tách chữ (3) < đảo hai chữ (4) < xoá/chèn/thay (6) < thay nguyên âm (8). Cộng 1
nếu số chữ đổi. Không phân ra thì các cách hoà giá nhau và ai thắng là do **thứ
tự bảng chữ cái** — `khôgn` ra `khôn`, `đing` ra `đang`.

**Thanh gốc của chữ không dấu là thanh ngang, không phải "không có".** Để `None`
thì mọi thanh đều bị tính là khác thanh gốc và `đnag` cho ra `đang`, `đàng`,
`đáng` hoà nhau hết.

**Phông đi kèm egui không có chữ Việt.** Nó thiếu khối Latin Extended Additional
(U+1EA0–U+1EF9) — hơn nửa số chữ có dấu. `main.rs` mượn phông hệ điều hành.
`tests/phong_chu.rs` quét **mọi ký tự ngoài ASCII trong chuỗi của mã nguồn giao
diện** và kiểm phông có glyph — đã bắt được `▸` hiện ra ô vuông trống. Đừng dùng
ký hiệu ngoài Latin-1 trong chuỗi giao diện.

**Sửa `du-lieu/*.txt` bằng tay là sai.** `am-tiet.txt` và `tu-ghep.txt` sinh ra
từ `crates/chinhta/examples/dung_tu_dien.rs`; sửa tay thì lần dựng lại sau mất
hết. Xem `du-lieu/NGUON.md`.

**Thêm luật mới thì đo lại bằng `soi_van.rs`.** Nó quét chín triệu tiếng trong
sách thật và liệt kê những tiếng bị coi là sai, xếp theo số lần gặp — mục nào lặp
lại nhiều lần mà bị báo sai thì gần như chắc chắn là lỗi của ta, không phải của
sách.

```bash
cargo run --release -p giaodien --example soi_van -- sach1.epub sach2.epub
```

## Số đo hiện tại

Harry Potter tập 4 (1,2 triệu chữ, 9.577 đoạn):

| | Không mô hình | Có mô hình 9B |
|---|---|---|
| Lỗi chữ nghĩa bắt được | 75 | 126 |
| Chỗ ngờ để lại | 86 | 0 |
| Thời gian | 4,4 giây | ~60 giây |

Bộ 13 câu mẫu (dính chữ, lỗi gõ, và các ca bẫy không được sửa): **13/13**.

Con số "lỗi chữ nghĩa" **không** tính khoảng trắng, Unicode, dấu câu và kiểu đặt
dấu. Gộp chúng vào thì cuốn nào cũng "hàng chục nghìn lỗi" mà phần lớn là rác
định dạng — và `hoá`/`hóa` thì cả hai đều đúng.

## Bản quyền

`du-lieu/am-tiet.txt` và `du-lieu/tu-ghep.txt` chắt ra từ ba bộ từ điển của người
khác (tudientv, Wiktionary tiếng Việt, Hồ Ngọc Đức) — xem `du-lieu/NGUON.md`.
File từ điển gốc không nằm trong repo.

# Ghim kiến trúc CUDA cần dựng.
#
# Không có file này thì llama.cpp dựng cho **cả bảy** kiến trúc mà CUDA 13 còn
# hỗ trợ (75, 80, 86, 89, 90, 120, 121) vì nó không biết máy có card gì. Mỗi
# kiến trúc là một lượt biên dịch riêng cho toàn bộ nhân CUDA, nên bảy lượt mất
# hàng chục phút — để dựng ra sáu bản mã mà máy này không bao giờ chạy tới.
#
# Cách hiển nhiên là bật `GGML_NATIVE` cho cmake tự dò card, nhưng không dùng
# được: `build.rs` của llama-cpp-sys-2 tự đặt `GGML_NATIVE=OFF` **sau** khi đọc
# biến môi trường, nên đặt biến ấy không ăn thua. Bật được thì phải mở
# `-C target-cpu=native` cho cả crate, mà thế thì binary chỉ chạy trên đúng đời
# CPU này.
#
# Nên chặn từ tầng dưới: cmake nạp file toolchain **trước** khi đọc CMakeLists,
# nên `CMAKE_CUDA_ARCHITECTURES` đã có giá trị lúc llama.cpp kiểm
# `if (NOT DEFINED CMAKE_CUDA_ARCHITECTURES)`, và nó nhường.
#
# `120a-real`: 120 là Blackwell (RTX 50xx). Hậu tố `a` là bản có thêm lệnh riêng
# của kiến trúc, `-real` là chỉ sinh mã máy chứ không kèm PTX — không cần PTX vì
# ta biết chính xác card sẽ chạy.
#
# **Đổi card thì sửa dòng dưới.** Bảng tra: 89 = RTX 40xx, 86 = RTX 30xx,
# 75 = RTX 20xx, 120 = RTX 50xx. Hoặc xoá biến `CMAKE_TOOLCHAIN_FILE` trong
# `.cargo/config.toml` để quay về dựng đủ mọi kiến trúc.
if(NOT DEFINED CMAKE_CUDA_ARCHITECTURES)
  set(CMAKE_CUDA_ARCHITECTURES 120a-real CACHE STRING "kiến trúc CUDA cần dựng")
endif()

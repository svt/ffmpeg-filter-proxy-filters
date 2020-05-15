/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_IMAGE_H_
#define CEF_FILTER_IMAGE_H_

#include <cstdint>

#include "pixel_format.h"

// TODO(chrsan): SIMD

namespace {
constexpr std::uint32_t kMask = 0xFFFF;

void GetRGBA(const std::uint8_t* pixels,
             cef_filter::PixelFormat pixel_format,
             bool dst,
             std::uint32_t& red,
             std::uint32_t& green,
             std::uint32_t& blue,
             std::uint32_t& alpha) noexcept {
  green = pixels[1];
  green |= green << 8;
  alpha = pixels[3];
  if (pixel_format == cef_filter::PixelFormat::kRGBA) {
    red = pixels[0];
    blue = pixels[2];
  } else {
    blue = pixels[0];
    red = pixels[2];
  }

  red |= red << 8;
  blue |= blue << 8;

  if (dst) {
    red *= alpha;
    red /= 0xFF;
    green *= alpha;
    green /= 0xFF;
    blue *= alpha;
    blue /= 0xFF;
  }

  alpha |= alpha << 8;
}

void SetRGBA(std::uint8_t* pixels,
             cef_filter::PixelFormat pixel_format,
             std::uint32_t red,
             std::uint32_t green,
             std::uint32_t blue,
             std::uint32_t alpha) noexcept {
  if (alpha == 0) {
    *((std::uint32_t*)pixels) = 0;
    return;
  }

  if (alpha != kMask) {
    red = (red * kMask) / alpha;
    green = (green * kMask) / alpha;
    blue = (blue * kMask) / alpha;
  }

  pixels[1] = (green >> 8) & 0xFF;
  pixels[3] = (alpha >> 8) & 0xFF;

  red = (red >> 8) & 0xFF;
  blue = (blue >> 8) & 0xFF;
  if (pixel_format == cef_filter::PixelFormat::kRGBA) {
    pixels[0] = red;
    pixels[2] = blue;
  } else {
    pixels[0] = blue;
    pixels[2] = red;
  }
}
}  // namespace

namespace cef_filter {
void Draw(const int width,
          const int height,
          std::uint8_t* dst,
          const PixelFormat dst_format,
          const std::uint8_t* src) {
  for (int y = 0; y < height; ++y) {
    for (int x = 0; x < width; ++x) {
      int offset = (y * width * 4) + (x * 4);

      std::uint32_t src_red, src_green, src_blue, src_alpha;
      GetRGBA(src + offset, PixelFormat::kBGRA, false, src_red, src_green,
              src_blue, src_alpha);

      std::uint32_t dst_red, dst_green, dst_blue, dst_alpha;
      GetRGBA(dst + offset, dst_format, true, dst_red, dst_green, dst_blue,
              dst_alpha);

      std::uint32_t alpha = kMask - src_alpha;
      SetRGBA(dst + offset, dst_format,
              ((dst_red * alpha) + (src_red * kMask)) / kMask,
              ((dst_green * alpha) + (src_green * kMask)) / kMask,
              ((dst_blue * alpha) + (src_blue * kMask)) / kMask,
              ((dst_alpha * alpha) + (src_alpha * kMask)) / kMask);
    }
  }
}
}  // namespace cef_filter

#endif  // CEF_FILTER_IMAGE_H_

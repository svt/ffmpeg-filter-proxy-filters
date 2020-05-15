/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#include <cstdlib>
#include <future>
#include <iostream>
#include <regex>
#include <string>

#include "app.h"
#include "client.h"
#include "context.h"
#include "loader.h"
#include "messages.h"
#include "pixel_format.h"
#include "task.h"

namespace {
bool ParseConfig(const char* config,
                 std::string& url,
                 std::string& subprocess_path) {
  const std::regex re("^url=(.+);subprocess=(.+)$");

  std::cmatch match;
  if (std::regex_match(config, match, re)) {
    url = match[1].str();
    subprocess_path = match[2].str();
    return true;
  }

  return false;
}
}  // namespace

extern "C" {
__attribute__((visibility("default"))) int filter_init(const char* config,
                                                       int pixel_format,
                                                       void** user_data) {
  if (!config) {
    std::cerr << "got null config" << std::endl;
    return 1;
  }

  if (pixel_format < cef_filter::PixelFormat::kRGBA ||
      pixel_format > cef_filter::PixelFormat::kBGRA) {
    std::cerr << "invalid pixel format: " << pixel_format << std::endl;
    return 1;
  }

  std::string url;
  std::string subprocess_path;
  if (!ParseConfig(config, url, subprocess_path)) {
    std::cerr << "error parsing: " << config << std::endl;
    return 1;
  }

  const char* cef_root = std::getenv("CEF_ROOT");
  if (!cef_root) {
    std::cerr << "no CEF_ROOT in env" << std::endl;
    return 1;
  }

  cef_filter::Loader loader(cef_root);
  if (!loader.Load()) {
    std::cerr << "could not load CEF" << std::endl;
    return 1;
  }

  std::cout << "filter_init: url = " << url
            << ", subprocess_path = " << subprocess_path << std::endl;

  std::promise<void> promise;
  std::future<void> future = std::async(std::launch::async, [&] {
    CefMainArgs main_args;

    CefSettings settings;
    settings.no_sandbox = true;
    settings.windowless_rendering_enabled = true;
    settings.background_color = 0x00000000;
    settings.log_severity = cef_log_severity_t::LOGSEVERITY_INFO;

#ifdef OS_MACOSX
    CefString(&settings.framework_dir_path)
        .FromString(loader.cef_framework_dir());
#endif
    CefString(&settings.log_file).FromASCII("/dev/stdout");
    CefString(&settings.browser_subprocess_path).FromString(subprocess_path);

    CefRefPtr<cef_filter::App> app(new cef_filter::App(nullptr));
    CefInitialize(main_args, settings, app.get(), nullptr);

    promise.set_value();
    CefRunMessageLoop();
  });

  promise.get_future().wait();
  *user_data = new cef_filter::Context(
      url, static_cast<cef_filter::PixelFormat>(pixel_format),
      std::move(future));

  return 0;
}

__attribute__((visibility("default"))) int filter_frame(unsigned char* data,
                                                        unsigned int data_size,
                                                        int width,
                                                        int height,
                                                        int line_size,
                                                        double ts_millis,
                                                        void* user_data) {
  if (!user_data || width <= 0 || height <= 0) {
    return 0;
  }

  if (line_size != width * 4) {
    std::cerr << "invalid line size" << std::endl;
    cef_filter::QuitMessageLoop();
    return 1;
  }

  if ((int)data_size != height * line_size) {
    std::cerr << "invalid data size" << std::endl;
    cef_filter::QuitMessageLoop();
    return 1;
  }

  cef_filter::Context* ctx = static_cast<cef_filter::Context*>(user_data);
  if (!ctx->IsBrowserCreated()) {
    if (!ctx->CreateBrowser(width, height)) {
      std::cerr << "could not create browser for URL: " << ctx->url()
                << std::endl;
      cef_filter::QuitMessageLoop();
      return 1;
    }
  } else {
    ctx->client()->UpdateWidthAndHeight(width, height);
  }

  std::future<bool> tick_response = ctx->client()->SendTickMessage(ts_millis);
  bool animation_frames_requested = tick_response.get();
  if (!animation_frames_requested) {
    return 0;
  }

  ctx->client()->SetPaintState(data).wait();

  return 0;
}

__attribute__((visibility("default"))) void filter_uninit(void* user_data) {
  if (user_data) {
    cef_filter::Context* ctx = static_cast<cef_filter::Context*>(user_data);
    ctx->Quit();
    delete ctx;
    CefShutdown();
  }
}
}

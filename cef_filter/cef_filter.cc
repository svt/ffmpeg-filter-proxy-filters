/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#include "app.h"
#include "loader.h"

#include <cstdlib>
#include <future>
#include <iostream>
#include <regex>
#include <string>

#include <include/base/cef_bind.h>
#include <include/cef_client.h>
#include <include/cef_task.h>
#include <include/wrapper/cef_closure_task.h>

#include <cairo.h>

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

struct Client : public CefClient,
                public CefLifeSpanHandler,
                public CefLoadHandler,
                public CefRenderHandler {
  Client(std::promise<bool>&& loaded_promise, int width, int height)
      : loaded_promise_(std::move(loaded_promise)),
        width_(width),
        height_(height) {}

  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() noexcept override {
    return this;
  }

  CefRefPtr<CefLoadHandler> GetLoadHandler() noexcept override { return this; }

  CefRefPtr<CefRenderHandler> GetRenderHandler() noexcept override {
    return this;
  }

  void OnAfterCreated(CefRefPtr<CefBrowser> browser) noexcept override {
    browser_ = browser;
  }

  void OnBeforeClose(CefRefPtr<CefBrowser> browser) noexcept override {
    browser_ = nullptr;
  }

  void OnLoadEnd(CefRefPtr<CefBrowser> browser,
                 CefRefPtr<CefFrame> frame,
                 int httpStatusCode) noexcept override {
    loaded_promise_.set_value(true);
  }

  void OnLoadError(CefRefPtr<CefBrowser> browser,
                   CefRefPtr<CefFrame> frame,
                   ErrorCode errorCode,
                   const CefString& errorText,
                   const CefString& failedUrl) noexcept override {
    CefPostTask(TID_UI, base::Bind(CefQuitMessageLoop));
    loaded_promise_.set_value(false);
  }

  void GetViewRect(CefRefPtr<CefBrowser> browser,
                   CefRect& rect) noexcept override {
    rect.width = width_;
    rect.height = height_;
  }

  void OnPaint(CefRefPtr<CefBrowser> browser,
               PaintElementType type,
               const RectList& dirtyRects,
               const void* buffer,
               int width,
               int height) noexcept override {
    if (!paint_state_.waiting) {
      std::cout << "Client::OnPaint -> not waiting" << std::endl;
      return;
    }

    paint_state_.waiting = false;

    /*
    const unsigned char* src = static_cast<const unsigned char*>(buffer);
    unsigned char* dst = new unsigned char[width * height * 4];

    int offset = 0;
    for (int y = 0; y < height; ++y) {
      for (int x = 0; x < width; ++x) {
        dst[offset + 0] = src[offset + 2];
        dst[offset + 1] = src[offset + 1];
        dst[offset + 2] = src[offset + 0];
        dst[offset + 3] = src[offset + 3];

        offset += 4;
      }
    }
    */

    cairo_surface_t* dst_surface = cairo_image_surface_create_for_data(
        paint_state_.data, CAIRO_FORMAT_ARGB32, width, height, width * 4);
    /*
cairo_surface_t* src_surface = cairo_image_surface_create_for_data(
    dst, CAIRO_FORMAT_ARGB32, width, height, width * 4);
    */
    cairo_surface_t* src_surface = cairo_image_surface_create_for_data(
        (unsigned char*)buffer, CAIRO_FORMAT_ARGB32, width, height, width * 4);

    cairo_t* cr = cairo_create(dst_surface);
    cairo_set_source_surface(cr, src_surface, 0, 0);
    cairo_paint(cr);

    cairo_destroy(cr);
    cairo_surface_destroy(src_surface);
    cairo_surface_destroy(dst_surface);

    paint_state_.promise.set_value();
  }

  void SetPaintState(unsigned char* data, std::promise<void>&& promise) {
    paint_state_.data = data;
    paint_state_.promise = std::move(promise);
    paint_state_.waiting = true;
    browser_->GetHost()->Invalidate(PaintElementType::PET_VIEW);
  }

  void UpdateWidthAndHeight(int width, int height) {
    width_ = width;
    height_ = height;
  }

  void Close() noexcept {
    if (browser_.get()) {
      browser_->GetHost()->CloseBrowser(true);
    }
  }

  int width() const noexcept { return width_; }
  int height() const noexcept { return height_; }

  CefRefPtr<CefBrowser>& browser() noexcept { return browser_; }

 private:
  std::promise<bool> loaded_promise_;

  int width_;
  int height_;

  CefRefPtr<CefBrowser> browser_;

  struct PaintState {
    unsigned char* data;
    std::promise<void> promise;
    bool waiting = false;
  };

  PaintState paint_state_;

  IMPLEMENT_REFCOUNTING(Client);
};

class Context {
 public:
  Context(const std::string& url,
          std::future<void>&& initialized_future) noexcept
      : url_(url), initialized_future_(std::move(initialized_future)) {}

  bool CreateBrowser(int width, int height) noexcept {
    std::promise<bool> loaded_promise;
    std::future<bool> loaded_future = loaded_promise.get_future();

    client_ = new Client(std::move(loaded_promise), width, height);

    CefWindowInfo window_info;
    window_info.SetAsWindowless(nullptr);
    window_info.width = width;
    window_info.height = height;

    CefBrowserSettings browser_settings;
    browser_settings.web_security = STATE_DISABLED;
    browser_settings.windowless_frame_rate = 25;  // TODO(chrsan): Fix me!
    browser_settings.background_color = 0x00000000;

    CefBrowserHost::CreateBrowser(window_info, client_.get(), url_,
                                  browser_settings, nullptr, nullptr);

    return loaded_future.get();
  }

  void Quit() noexcept {
    CefPostTask(TID_UI, base::Bind(CefQuitMessageLoop));
    initialized_future_.wait();
  }

  const std::string& url() const noexcept { return url_; }

  CefRefPtr<Client>& client() noexcept { return client_; }

 private:
  std::string url_;
  std::future<void> initialized_future_;

  CefRefPtr<Client> client_;
};
}  // namespace

extern "C" {
__attribute__((visibility("default"))) int filter_init(const char* config,
                                                       void** user_data) {
  if (!config) {
    std::cerr << "got null config" << std::endl;
    return 1;
  }

  std::string url;
  std::string subprocess_path;
  if (!ParseConfig(config, url, subprocess_path)) {
    std::cerr << "error parsing: " << config << std::endl;
    return 1;
  }

  if (url.rfind("http://", 0) != 0 || url.rfind("https://", 0) != 0 ||
      url.rfind("file://", 0) != 0) {
    url = std::string("file://") + url;
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
  std::future<void> initialized_future = std::async(std::launch::async, [&] {
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

    CefRefPtr<cef_filter::App> app(new cef_filter::App);
    CefInitialize(main_args, settings, app.get(), nullptr);

    promise.set_value();
    CefRunMessageLoop();
  });

  promise.get_future().wait();
  *user_data = new Context(url, std::move(initialized_future));

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
    CefPostTask(TID_UI, base::Bind(CefQuitMessageLoop));
    return 1;
  }

  if ((int)data_size != height * line_size) {
    std::cerr << "invalid data size" << std::endl;
    CefPostTask(TID_UI, base::Bind(CefQuitMessageLoop));
    return 1;
  }

  Context* ctx = static_cast<Context*>(user_data);
  if (!ctx->client().get()) {
    if (!ctx->CreateBrowser(width, height)) {
      std::cerr << "could not create browser for URL: " << ctx->url()
                << std::endl;
      return 1;
    }
  } else {
    ctx->client()->UpdateWidthAndHeight(width, height);
  }

  CefRefPtr<CefBrowser>& browser = ctx->client()->browser();
  browser->GetMainFrame()->ExecuteJavaScript(
      std::string("animateCircle(") + std::to_string(ts_millis) + ");",
      browser->GetMainFrame()->GetURL(), 0);

  std::promise<void> paint_promise;
  std::future<void> paint_future = paint_promise.get_future();
  ctx->client()->SetPaintState(data, std::move(paint_promise));
  paint_future.wait();

  return 0;
}

__attribute__((visibility("default"))) void filter_uninit(void* user_data) {
  if (user_data) {
    Context* ctx = static_cast<Context*>(user_data);
    if (ctx->client().get()) {
      ctx->client()->Close();
    }

    ctx->Quit();
    delete ctx;

    CefShutdown();
  }
}
}

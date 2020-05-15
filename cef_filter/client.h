/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_CLIENT_H_
#define CEF_FILTER_CLIENT_H_

#include <include/cef_app.h>
#include <include/cef_client.h>

#include <future>
#include <string>

#include "image.h"
#include "messages.h"
#include "pixel_format.h"
#include "task.h"

namespace cef_filter {
class Client : public CefClient,
               public CefLifeSpanHandler,
               public CefLoadHandler,
               public CefRenderHandler {
 public:
  explicit Client(PixelFormat pixel_format) : pixel_format_(pixel_format) {}

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
    loaded_promise_.set_value(false);
  }

  void GetViewRect(CefRefPtr<CefBrowser> browser,
                   CefRect& rect) noexcept override {
    rect.Set(0, 0, width_, height_);
  }

  void OnPaint(CefRefPtr<CefBrowser> browser,
               PaintElementType type,
               const RectList& dirtyRects,
               const void* buffer,
               int width,
               int height) noexcept override {
    if (type != PaintElementType::PET_VIEW || !paint_state_.waiting) {
      return;
    }

    paint_state_.waiting = false;
    Draw(width, height, static_cast<std::uint8_t*>(paint_state_.buffer),
         pixel_format_, static_cast<const std::uint8_t*>(buffer));
    paint_state_.promise.set_value();
  }

  bool OnProcessMessageReceived(
      CefRefPtr<CefBrowser> browser,
      CefRefPtr<CefFrame> frame,
      CefProcessId source_process,
      CefRefPtr<CefProcessMessage> message) noexcept override {
    if (message->GetName() == kTickResponse) {
      tick_response_promise_.set_value(message->GetArgumentList()->GetBool(0));
      return true;
    }

    return false;
  }

  void UpdateWidthAndHeight(int width, int height) noexcept {
    width_ = width;
    height_ = height;
  }

  std::future<bool> SendTickMessage(double ts_millis) noexcept {
    tick_response_promise_ = std::promise<bool>();

    CefRefPtr<CefProcessMessage> tick_request =
        CefProcessMessage::Create(cef_filter::kTickRequest);
    tick_request->GetArgumentList()->SetDouble(0, ts_millis);

    browser_->GetMainFrame()->SendProcessMessage(PID_RENDERER,
                                                 std::move(tick_request));

    return tick_response_promise_.get_future();
  }

  std::future<void> SetPaintState(unsigned char* buffer) noexcept {
    paint_state_.waiting = true;
    paint_state_.buffer = buffer;
    paint_state_.promise = std::promise<void>();

    browser_->GetHost()->Invalidate(PaintElementType::PET_VIEW);

    return paint_state_.promise.get_future();
  }

  void Close() noexcept {
    if (browser_.get()) {
      browser_->GetHost()->CloseBrowser(true);
    }
  }

  std::future<bool> LoadedFuture() noexcept {
    return loaded_promise_.get_future();
  }

  std::future<bool> TickResponseFuture() noexcept {
    return tick_response_promise_.get_future();
  }

  int width() const noexcept { return width_; }
  int height() const noexcept { return height_; }

  CefRefPtr<CefBrowser>& browser() noexcept { return browser_; }

 private:
  PixelFormat pixel_format_;

  std::promise<bool> loaded_promise_;
  std::promise<bool> tick_response_promise_;

  int width_;
  int height_;

  CefRefPtr<CefBrowser> browser_;

  struct PaintState {
    unsigned char* buffer = nullptr;
    bool waiting = false;
    std::promise<void> promise;
  };

  PaintState paint_state_;

  IMPLEMENT_REFCOUNTING(Client);
  DISALLOW_COPY_AND_ASSIGN(Client);
};
}  // namespace cef_filter

#endif  // CEF_FILTER_CLIENT_H_

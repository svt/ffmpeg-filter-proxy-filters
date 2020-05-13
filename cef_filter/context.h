/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_CONTEXT_H_
#define CEF_FILTER_CONTEXT_H_

#include <chrono>
#include <future>
#include <string>

#include "client.h"
#include "task.h"

namespace cef_filter {
class Context {
 public:
  Context(const std::string& url,
          std::future<void>&& cef_message_loop_future) noexcept
      : url_(url),
        cef_message_loop_future_(std::move(cef_message_loop_future)),
        client_(new Client) {}

  bool IsBrowserCreated() const noexcept { return client_->browser().get(); }

  bool CreateBrowser(int width, int height) noexcept {
    client_->UpdateWidthAndHeight(width, height);

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

    std::future<bool> future = client_->LoadedFuture();
    std::future_status status = future.wait_for(std::chrono::seconds(5));
    if (status != std::future_status::ready) {
      return false;
    }

    return future.get();
  }

  void Quit() noexcept {
    client_->Close();
    cef_filter::QuitMessageLoop();
    cef_message_loop_future_.wait();
  }

  const std::string& url() const noexcept { return url_; }
  CefRefPtr<Client>& client() noexcept { return client_; }

 private:
  std::string url_;
  std::future<void> cef_message_loop_future_;
  CefRefPtr<Client> client_;
};
}  // namespace cef_filter

#endif  // CEF_FILTER_CONTEXT_H_

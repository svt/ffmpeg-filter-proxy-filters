/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_APP_H_
#define CEF_FILTER_APP_H_

#include <include/cef_app.h>

namespace cef_filter {
class App : public CefApp {
 public:
  explicit App(CefRefPtr<CefRenderProcessHandler>&& render_process_handler)
      : render_process_handler_(std::move(render_process_handler)) {}

  void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) noexcept override {
    if (process_type.empty()) {
      command_line->AppendSwitch("disable-gpu-program-cache");
      command_line->AppendSwitch("disable-gpu-shader-disk-cache");

#ifdef OS_MACOSX
      command_line->AppendSwitch("use-mock-keychain");
#endif
    }
  }

  CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() override {
    return render_process_handler_;
  }

 private:
  CefRefPtr<CefRenderProcessHandler> render_process_handler_;

  IMPLEMENT_REFCOUNTING(App);
  DISALLOW_COPY_AND_ASSIGN(App);
};
}  // namespace cef_filter

#endif  // CEF_FILTER_APP_H_
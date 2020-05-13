/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_RENDER_PROCESS_HANDLER_H_
#define CEF_FILTER_RENDER_PROCESS_HANDLER_H_

#include <include/cef_render_process_handler.h>

#include <algorithm>
#include <string>
#include <vector>

#include "messages.h"

namespace {
constexpr const char kJavaScript[] = R"(
    (function() {
      const ctx = {
        requestedAnimationFrames: new Map(),
        currentAnimationFrameId: 0,
        tick: function(ts) {
          if (this.requestedAnimationFrames.size === 0) {
            return false;
          }

          const requestedFrames = new Map(this.requestedAnimationFrames);
          this.requestedAnimationFrames.clear();
          requestedFrames.forEach((callback) => {
            callback(ts);
          });

          return true;
        },
      };

      ctx.tick = ctx.tick.bind(ctx);

      window.requestAnimationFrame = (callback) => {
        ctx.requestedAnimationFrames.set(++ctx.currentAnimationFrameId, callback);
      };

      window.cancelAnimationFrame = (id) => {
        ctx.requestedAnimationFrames.delete(id);
      };

      window["__CEF_FILTER__"] = ctx;
    })();
  )";
}

namespace cef_filter {
class RenderProcessHandler : public CefRenderProcessHandler {
 public:
  void OnBrowserDestroyed(CefRefPtr<CefBrowser> browser) override {
    contexts_.clear();
  }

  void OnContextCreated(CefRefPtr<CefBrowser> browser,
                        CefRefPtr<CefFrame> frame,
                        CefRefPtr<CefV8Context> context) override {
    contexts_.push_back(context);

    CefRefPtr<CefV8Value> global = context->GetGlobal();

    CefRefPtr<CefV8Value> return_value;
    CefRefPtr<CefV8Exception> exception;
    context->Eval(kJavaScript, CefString(), 1, return_value, exception);
  }

  void OnContextReleased(CefRefPtr<CefBrowser> browser,
                         CefRefPtr<CefFrame> frame,
                         CefRefPtr<CefV8Context> context) override {
    contexts_.erase(std::remove_if(contexts_.begin(), contexts_.end(),
                                   [&](const CefRefPtr<CefV8Context>& c) {
                                     return c->IsSame(context);
                                   }),
                    contexts_.end());
  }

  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                CefProcessId source_process,
                                CefRefPtr<CefProcessMessage> message) override {
    if (message->GetName() == cef_filter::kTickRequest) {
      double ts = message->GetArgumentList()->GetDouble(0);
      const CefString js =
          "window[\"__CEF_FILTER__\"].tick(" + std::to_string(ts) + ");";

      bool animation_frames_requested = false;
      for (const CefRefPtr<CefV8Context>& context : contexts_) {
        CefRefPtr<CefV8Value> return_val;
        CefRefPtr<CefV8Exception> exception;
        if (context->Eval(js, CefString(), 1, return_val, exception)) {
          if (return_val->GetBoolValue()) {
            animation_frames_requested = true;
          }
        }
      }

      CefRefPtr<CefProcessMessage> tick_response =
          CefProcessMessage::Create(cef_filter::kTickResponse);
      tick_response->GetArgumentList()->SetBool(0, animation_frames_requested);

      browser->GetMainFrame()->SendProcessMessage(PID_BROWSER,
                                                  std::move(tick_response));

      return true;
    }

    return false;
  }

 private:
  std::vector<CefRefPtr<CefV8Context>> contexts_;

  IMPLEMENT_REFCOUNTING(RenderProcessHandler);
};
}  // namespace cef_filter

#endif  // CEF_FILTER_RENDER_PROCESS_HANDLER_H_

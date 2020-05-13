/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_TASK_H_
#define CEF_FILTER_TASK_H_

#include <include/base/cef_bind.h>
#include <include/cef_app.h>
#include <include/cef_task.h>
#include <include/wrapper/cef_closure_task.h>

#include <functional>
#include <future>

namespace cef_filter {
void QuitMessageLoop() noexcept {
  CefPostTask(TID_UI, base::Bind(CefQuitMessageLoop));
}
}  // namespace cef_filter

#endif  // CEF_FILTER_TASK_H_

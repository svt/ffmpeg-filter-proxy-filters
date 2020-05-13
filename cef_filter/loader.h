/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#ifndef CEF_FILTER_LOADER_H_
#define CEF_FILTER_LOADER_H_

#include <include/base/cef_macros.h>
#include <include/wrapper/cef_library_loader.h>

#include <sstream>
#include <string>

namespace cef_filter {
class Loader {
 public:
  Loader(const std::string& cef_root) : cef_root_(cef_root), loaded_(false) {}

  ~Loader() {
    if (loaded_) {
      cef_unload_library();
    }
  }

  bool Load() {
    if (loaded_) {
      return true;
    }

    if (cef_root_.empty()) {
      return false;
    }

    std::stringstream ss;
    ss << cef_root_;
    if (cef_root_[cef_root_.size() - 1] != '/') {
      ss << "/";
    }

#ifndef OS_MACOSX
    ss << "libcef.so";
#else
    ss << "Release/Chromium Embedded Framework.framework";
    cef_framework_dir_ = ss.str();
    ss << "/Chromium Embedded Framework";
#endif
    if (!cef_load_library(ss.str().c_str())) {
      return false;
    }

    loaded_ = true;
    return true;
  }

  const std::string& cef_root() const noexcept { return cef_root_; }

#ifdef OS_MACOSX
  const std::string& cef_framework_dir() const noexcept {
    return cef_framework_dir_;
#endif
  }

 private:
  std::string cef_root_;
  bool loaded_;

#ifdef OS_MACOSX
  std::string cef_framework_dir_;
#endif

  DISALLOW_COPY_AND_ASSIGN(Loader);
};  // namespace cef_filter
}  // namespace cef_filter

#endif  // CEF_FILTER_LOADER_H_

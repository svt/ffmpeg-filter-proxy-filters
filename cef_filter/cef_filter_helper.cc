/*
 * SPDX-FileCopyrightText: 2020 Sveriges Television AB
 *
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */

#include <cstdlib>
#include <iostream>

#include <include/cef_app.h>

#include "app.h"
#include "loader.h"

int main(int argc, char* argv[]) {
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

  CefMainArgs main_args(argc, argv);
  CefRefPtr<cef_filter::App> app(new cef_filter::App);
  return CefExecuteProcess(main_args, app.get(), nullptr);
}

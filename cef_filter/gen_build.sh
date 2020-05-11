#!/bin/bash

if [[ ! -d $CEF_ROOT ]]; then
  echo "Be sure to set the CEF_ROOT environment variable"
  exit 1
fi

BASE_DIR=`cd $(dirname ${BASH_SOURCE[0]}) && pwd`

mkdir -p ${BASE_DIR}/build
pushd ${BASE_DIR}/build
cmake -G "Ninja" -DPROJECT_ARCH="x86_64" -DCMAKE_BUILD_TYPE=Release -DUSE_SANDBOX=OFF ..

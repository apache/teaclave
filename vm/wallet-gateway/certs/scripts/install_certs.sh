# Licensed to the Apache Software Foundation (ASF) under one
# or more contributor license agreements.  See the NOTICE file
# distributed with this work for additional information
# regarding copyright ownership.  The ASF licenses this file
# to you under the Apache License, Version 2.0 (the
# "License"); you may not use this file except in compliance
# with the License.  You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing,
# software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
# KIND, either express or implied.  See the License for the
# specific language governing permissions and limitations
# under the License.

#!/bin/bash

set -e

CK_CLIENT_DB_ROOT_PATH=$1
CK_TEE_BUILD_PATH=$2
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
TMP=$SCRIPT_DIR/tmp

# check if paths are set
if [ -z "$CK_CLIENT_DB_ROOT_PATH" ] || [ -z "$CK_TEE_BUILD_PATH" ]; then
  echo "Usage: install_certs.sh <ck-root-path> <ck-tee-build-path>"
  exit 1
fi
if [ ! -d $TMP ]; then
  echo "$TMP does not exist. Please run generate_certs.sh first."
  exit 1 
fi

# install certs into ck client storage
CK_ROOT_CERTS_PATH=$CK_CLIENT_DB_ROOT_PATH/certs
if [ ! -d $CK_ROOT_CERTS_PATH ]; then
  mkdir $CK_ROOT_CERTS_PATH
fi
cp $TMP/ca.key $TMP/ca.cert $TMP/ca.der $CK_ROOT_CERTS_PATH
cp $TMP/system.key $CK_ROOT_CERTS_PATH
echo "CA certificate (for authority) and system key pair (for webapi) installed into $CK_ROOT_CERTS_PATH"

# install pubkeys into tee part
CK_RS_CERTS_PATH=$CK_TEE_BUILD_PATH/pubkeys
if [ ! -d $CK_RS_CERTS_PATH ]; then
  mkdir $CK_RS_CERTS_PATH
fi
cp $TMP/ca.cert $TMP/ca.pub $TMP/system.pub $CK_RS_CERTS_PATH
echo "CA pubkey (for authority) and system pubkey (for webapi) installed into $CK_RS_CERTS_PATH"

echo "Certs installed"

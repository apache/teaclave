#!/bin/bash

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

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
TMP_DIR=$SCRIPT_DIR/tmp
if [ -d $TMP_DIR ]; then
  echo "Temporary directory already exists"
  exit 0
fi
mkdir $TMP_DIR

openssl ecparam -name secp384r1 -out $TMP_DIR/ca.pem
openssl req -nodes \
          -x509 \
          -newkey ec:$TMP_DIR/ca.pem \
          -keyout $TMP_DIR/ca.key \
          -out $TMP_DIR/ca.cert \
          -sha256 \
          -batch \
          -days 3650 \
          -subj "/CN=CK ECDSA CA"
openssl asn1parse -in $TMP_DIR/ca.cert -out $TMP_DIR/ca.der > /dev/null
openssl ec -in $TMP_DIR/ca.key -pubout -outform DER -out $TMP_DIR/ca.pub.der
PUBKEY=$(openssl pkey -pubin -inform DER -in $TMP_DIR/ca.pub.der -text -noout)
UNCOMPRESSED=$(echo $PUBKEY | sed ':a;N;$!ba;s/\n//g' | sed -e 's/[[:space:]]//g' | grep -o -E '04(:[[:xdigit:]]+)+')
echo $UNCOMPRESSED | xxd -r -p > $TMP_DIR/ca.pub
echo "CA certificate (for authority) generated"

openssl ecparam -genkey -name prime256v1 -noout -out $TMP_DIR/system.pem
openssl pkcs8 -topk8 -inform PEM -in $TMP_DIR/system.pem -out $TMP_DIR/system.key -nocrypt
openssl ec -in $TMP_DIR/system.pem -pubout -outform DER -out $TMP_DIR/system.pub.der
PUBKEY=$(openssl pkey -pubin -inform DER -in $TMP_DIR/system.pub.der -text -noout)
UNCOMPRESSED=$(echo $PUBKEY | sed ':a;N;$!ba;s/\n//g' | sed -e 's/[[:space:]]//g' | grep -o -E '04(:[[:xdigit:]]+)+')
echo $UNCOMPRESSED | xxd -r -p > $TMP_DIR/system.pub
echo "System key pair (for webapi) generated"

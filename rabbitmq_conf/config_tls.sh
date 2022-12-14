#!/bin/bash

COMMON_NAME=AMQPRS_TEST

# generate tls cert/key
#------------------------
git clone https://github.com/rabbitmq/tls-gen tls-gen
cd tls-gen/basic
make CN=$COMMON_NAME
make verify CN=$COMMON_NAME
make info CN=$COMMON_NAME
ls -lha ./result
cd -
sudo cp tls-gen/basic/result/* tls-test
rm -rf tls-gen

# to make sure the cert/key files have correct permissions
# and owners within container after bind mount 
#------------------------

# `1001` is the default user of `bitnami/rabbitmq` container
sudo chown -R 1001:root tls-test
# strict permissions is mandatory for TLS cert/key files
sudo chmod 755 tls-test
sudo chmod 400 tls-test/*

# verify tls connection
echo "---------- Start rabbitmq now, then come back ... ---------------"
read -p "After rabbitmq started, press 'y' to verify TLS connection: " ans
if [ "$ans" = "y" ]; then
    cd tls-test 
    sudo openssl s_client -connect localhost:5671 -cert client_${COMMON_NAME}_certificate.pem -key client_${COMMON_NAME}_key.pem -CAfile ca_certificate.pem
fi
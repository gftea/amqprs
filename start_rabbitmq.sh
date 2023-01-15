#!/bin/bash

COMMON_NAME=AMQPRS_TEST

# Create directories for rabbitmq server and client
#------------------------
mkdir -p rabbitmq_conf/server
mkdir -p rabbitmq_conf/client

# generate tls cert/key
#------------------------
git clone https://github.com/rabbitmq/tls-gen tls-gen
cd tls-gen/basic
make CN=$COMMON_NAME
make verify CN=$COMMON_NAME
make info CN=$COMMON_NAME
ls -lha ./result
cd -

# copy client files
sudo cp tls-gen/basic/result/ca_* rabbitmq_conf/client
sudo cp tls-gen/basic/result/client_* rabbitmq_conf/client
# copy server files
sudo cp tls-gen/basic/result/ca_* rabbitmq_conf/server
sudo cp tls-gen/basic/result/server_* rabbitmq_conf/server
# clean up
rm -rf tls-gen

# to make sure the cert/key files have correct permissions
# and owners within container after bind mount 
#------------------------

# copy server files to temparory folder for test
# `1001` is the default user of `bitnami/rabbitmq` container
sudo chown -R 1001:root rabbitmq_conf/server
# strict permissions is mandatory for TLS cert/key files
sudo chmod 755 rabbitmq_conf/server
sudo chmod 400 rabbitmq_conf/server/*

# start rabbitmq server
docker-compose down
docker-compose up -d

# # verify tls connection
# echo "---------- Start rabbitmq now, then come back ... ---------------"
# read -p "After rabbitmq started, press 'y' to verify TLS connection: " ans
# if [ "$ans" = "y" ]; then
#     cd rabbitmq_conf/client
#     openssl s_client -connect localhost:5671 -cert client_${COMMON_NAME}_certificate.pem -key client_${COMMON_NAME}_key.pem -CAfile ca_certificate.pem
# fi
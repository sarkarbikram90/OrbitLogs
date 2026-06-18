#!/bin/bash
set -e

# Directory for certs
mkdir -p /secrets/certs
cd /secrets/certs

# Check if certs already exist to avoid regeneration
if [ -f ca-cert ] && [ -f kafka-1.keystore.jks ] && [ -f client-cert.pem ]; then
  echo "Certificates already exist. Skipping certificate generation."
  exit 0
fi

echo "Generating mutual TLS certificates for 3-node Kafka cluster..."

# 1. Generate CA private key and certificate
openssl req -new -x509 -keyout ca-key -out ca-cert -days 365 -subj "/CN=SpaceOps-CA/OU=GroundSegment/O=SpaceOps/L=Earth/ST=State/C=US" -nodes

# 2. For each broker (kafka-1, kafka-2, kafka-3), generate keystore and truststore
for i in 1 2 3; do
  echo "Generating keystores and truststores for kafka-$i..."
  
  # Create Keystore
  keytool -keystore kafka-$i.keystore.jks -alias kafka-$i -validity 365 -genkey -keyalg RSA -keysize 2048 -dname "CN=kafka-$i,OU=GroundSegment,O=SpaceOps,L=Earth,S=State,C=US" -storepass spaceopspass -keypass spaceopspass

  # Export certificate request
  keytool -keystore kafka-$i.keystore.jks -alias kafka-$i -certreq -file kafka-$i.csr -storepass spaceopspass

  # Sign with CA
  openssl x509 -req -CA ca-cert -CAkey ca-key -in kafka-$i.csr -out kafka-$i-ca-signed -days 365 -CAcreateserial -passin pass:spaceopspass

  # Import CA root cert to keystore
  keytool -keystore kafka-$i.keystore.jks -alias CARoot -import -noprompt -file ca-cert -storepass spaceopspass

  # Import signed cert to keystore
  keytool -keystore kafka-$i.keystore.jks -alias kafka-$i -import -file kafka-$i-ca-signed -storepass spaceopspass

  # Create Truststore
  keytool -keystore kafka-$i.truststore.jks -alias CARoot -import -noprompt -file ca-cert -storepass spaceopspass
done

# 3. Create client certificate and key in PEM format for Rust clients
echo "Generating client certificates for Rust services..."
openssl req -new -newkey rsa:2048 -keyout client-key.pem -out client-req.pem -subj "/CN=spaceops-client/OU=GroundSegment/O=SpaceOps/L=Earth/ST=State/C=US" -nodes
openssl x509 -req -CA ca-cert -CAkey ca-key -in client-req.pem -out client-cert.pem -days 365 -CAcreateserial

# Clean up temporary files
rm -f *.csr *.srl kafka-*-ca-signed client-req.pem

echo "Mutual TLS certificates generated successfully in /secrets/certs!"

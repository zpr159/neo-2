#include <gtest/gtest.h>
#include <neo/security/crypto.hpp>

using namespace neo::security;

TEST(CryptoServiceTest, EncryptDecrypt) {
    auto crypto = CryptoService::create();
    std::string key = "secret_key_123";
    std::vector<uint8_t> plaintext = {72, 101, 108, 108, 111};

    auto ciphertext = crypto.encrypt(plaintext, key);
    EXPECT_EQ(ciphertext.size(), plaintext.size());
    EXPECT_NE(ciphertext, plaintext);

    auto decrypted = crypto.decrypt(ciphertext, key);
    EXPECT_EQ(decrypted, plaintext);
}

TEST(CryptoServiceTest, EncryptEmptyKey) {
    auto crypto = CryptoService::create();
    std::vector<uint8_t> data = {1, 2, 3};
    EXPECT_THROW(crypto.encrypt(data, ""), neo::core::Error);
    EXPECT_THROW(crypto.decrypt(data, ""), neo::core::Error);
}

TEST(CryptoServiceTest, HashSHA256) {
    auto crypto = CryptoService::create();
    std::string hash1 = crypto.hash("hello");
    std::string hash2 = crypto.hash("hello");
    std::string hash3 = crypto.hash("world");

    EXPECT_EQ(hash1.size(), 64u);
    EXPECT_EQ(hash1, hash2);
    EXPECT_NE(hash1, hash3);
}

TEST(CryptoServiceTest, HashSHA512) {
    auto crypto = CryptoService::create();
    std::string hash = crypto.hash("test", HashAlgorithm::SHA512);
    EXPECT_EQ(hash.size(), 128u);
}

TEST(CryptoServiceTest, HashMD5) {
    auto crypto = CryptoService::create();
    std::string hash = crypto.hash("test", HashAlgorithm::MD5);
    EXPECT_EQ(hash.size(), 32u);
}

TEST(CryptoServiceTest, SignVerify) {
    auto crypto = CryptoService::create();
    std::string data = "important message";
    std::string key = "private_key";

    auto signature = crypto.sign(data, key);
    EXPECT_TRUE(crypto.verify(data, signature, key));
    EXPECT_FALSE(crypto.verify("tampered", signature, key));
}

TEST(CryptoServiceTest, GenerateKey) {
    auto crypto = CryptoService::create();
    std::string key1 = crypto.generate_key(32);
    std::string key2 = crypto.generate_key(32);

    EXPECT_EQ(key1.size(), 64u);
    EXPECT_NE(key1, key2);
}

TEST(CryptoServiceTest, Base64EncodeDecode) {
    auto crypto = CryptoService::create();
    std::vector<uint8_t> data = {72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100};

    std::string encoded = crypto.base64_encode(data);
    auto decoded = crypto.base64_decode(encoded);
    EXPECT_EQ(decoded, data);
}

TEST(CryptoServiceTest, HashAlgorithmString) {
    EXPECT_STREQ(to_string(HashAlgorithm::SHA256), "SHA256");
    EXPECT_STREQ(to_string(HashAlgorithm::SHA512), "SHA512");
    EXPECT_STREQ(to_string(HashAlgorithm::MD5), "MD5");
}

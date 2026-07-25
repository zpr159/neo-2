#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace neo::security {

enum class HashAlgorithm : std::uint8_t {
    SHA256 = 0,
    SHA512 = 1,
    MD5 = 2
};

[[nodiscard]] const char* to_string(HashAlgorithm algo) noexcept;

class CryptoService {
public:
    CryptoService() = default;
    ~CryptoService() = default;

    CryptoService(const CryptoService&) = default;
    CryptoService& operator=(const CryptoService&) = default;
    CryptoService(CryptoService&&) noexcept = default;
    CryptoService& operator=(CryptoService&&) noexcept = default;

    [[nodiscard]] static CryptoService create();

    [[nodiscard]] std::vector<uint8_t> encrypt(const std::vector<uint8_t>& plaintext, const std::string& key);
    [[nodiscard]] std::vector<uint8_t> decrypt(const std::vector<uint8_t>& ciphertext, const std::string& key);

    [[nodiscard]] std::string hash(const std::string& data, HashAlgorithm algo = HashAlgorithm::SHA256);
    [[nodiscard]] std::vector<uint8_t> hash_bytes(const std::vector<uint8_t>& data, HashAlgorithm algo = HashAlgorithm::SHA256);

    [[nodiscard]] std::vector<uint8_t> sign(const std::string& data, const std::string& private_key);
    [[nodiscard]] bool verify(const std::string& data, const std::vector<uint8_t>& signature, const std::string& public_key);

    [[nodiscard]] std::string generate_key(std::size_t length = 32);
    [[nodiscard]] std::string base64_encode(const std::vector<uint8_t>& data);
    [[nodiscard]] std::vector<uint8_t> base64_decode(const std::string& encoded);

private:
    std::vector<uint8_t> xor_cipher(const std::vector<uint8_t>& data, const std::string& key);
    std::string hex_encode(const std::vector<uint8_t>& data);
};

} // namespace neo::security

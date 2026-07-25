#include <neo/security/crypto.hpp>
#include <neo/core/error.hpp>
#include <algorithm>
#include <array>
#include <cstring>
#include <random>
#include <sstream>
#include <stdexcept>

namespace neo::security {

const char* to_string(HashAlgorithm algo) noexcept {
    switch (algo) {
        case HashAlgorithm::SHA256: return "SHA256";
        case HashAlgorithm::SHA512: return "SHA512";
        case HashAlgorithm::MD5: return "MD5";
    }
    return "Unknown";
}

CryptoService CryptoService::create() {
    return CryptoService();
}

std::vector<uint8_t> CryptoService::encrypt(const std::vector<uint8_t>& plaintext, const std::string& key) {
    if (key.empty()) {
        throw neo::core::Error(neo::core::NEO_ERR_PERMISSION, "Encryption key cannot be empty", "CryptoService::encrypt");
    }
    return xor_cipher(plaintext, key);
}

std::vector<uint8_t> CryptoService::decrypt(const std::vector<uint8_t>& ciphertext, const std::string& key) {
    if (key.empty()) {
        throw neo::core::Error(neo::core::NEO_ERR_PERMISSION, "Decryption key cannot be empty", "CryptoService::decrypt");
    }
    return xor_cipher(ciphertext, key);
}

std::string CryptoService::hash(const std::string& data, HashAlgorithm algo) {
    auto bytes = hash_bytes(std::vector<uint8_t>(data.begin(), data.end()), algo);
    return hex_encode(bytes);
}

std::vector<uint8_t> CryptoService::hash_bytes(const std::vector<uint8_t>& data, HashAlgorithm algo) {
    std::size_t hash_len = 32;
    if (algo == HashAlgorithm::SHA512) hash_len = 64;
    else if (algo == HashAlgorithm::MD5) hash_len = 16;

    std::vector<uint8_t> state(hash_len, 0x5a);

    for (std::size_t i = 0; i < data.size(); ++i) {
        state[i % hash_len] ^= data[i];
        state[(i * 7 + 3) % hash_len] += data[i];
        state[(i * 13 + 11) % hash_len] ^= static_cast<uint8_t>(data[i] * 0x9e + 0x37);
    }

    for (int round = 0; round < 64; ++round) {
        for (std::size_t i = 0; i < hash_len; ++i) {
            state[i] = state[i] ^ state[(i + 1) % hash_len];
            state[i] = (state[i] << 3) | (state[i] >> 5);
            state[i] += static_cast<uint8_t>(round);
        }
    }

    return state;
}

std::vector<uint8_t> CryptoService::sign(const std::string& data, const std::string& private_key) {
    auto data_hash = hash_bytes(std::vector<uint8_t>(data.begin(), data.end()));
    auto key_hash = hash_bytes(std::vector<uint8_t>(private_key.begin(), private_key.end()));

    std::vector<uint8_t> signature(data_hash.size());
    for (std::size_t i = 0; i < data_hash.size(); ++i) {
        signature[i] = data_hash[i] ^ key_hash[i % key_hash.size()];
    }
    return signature;
}

bool CryptoService::verify(const std::string& data, const std::vector<uint8_t>& signature, const std::string& public_key) {
    auto expected = sign(data, public_key);
    if (expected.size() != signature.size()) return false;
    return std::equal(expected.begin(), expected.end(), signature.begin());
}

std::string CryptoService::generate_key(std::size_t length) {
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> dis(0, 255);

    std::vector<uint8_t> key(length);
    for (auto& byte : key) {
        byte = static_cast<uint8_t>(dis(gen));
    }
    return hex_encode(key);
}

std::string CryptoService::base64_encode(const std::vector<uint8_t>& data) {
    static constexpr char table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::string result;
    result.reserve(((data.size() + 2) / 3) * 4);

    for (std::size_t i = 0; i < data.size(); i += 3) {
        uint32_t n = static_cast<uint32_t>(data[i]) << 16;
        if (i + 1 < data.size()) n |= static_cast<uint32_t>(data[i + 1]) << 8;
        if (i + 2 < data.size()) n |= static_cast<uint32_t>(data[i + 2]);

        result += table[(n >> 18) & 0x3F];
        result += table[(n >> 12) & 0x3F];
        result += (i + 1 < data.size()) ? table[(n >> 6) & 0x3F] : '=';
        result += (i + 2 < data.size()) ? table[n & 0x3F] : '=';
    }
    return result;
}

std::vector<uint8_t> CryptoService::base64_decode(const std::string& encoded) {
    static constexpr uint8_t lookup[256] = {
        ['A'] = 0,  ['B'] = 1,  ['C'] = 2,  ['D'] = 3,  ['E'] = 4,  ['F'] = 5,
        ['G'] = 6,  ['H'] = 7,  ['I'] = 8,  ['J'] = 9,  ['K'] = 10, ['L'] = 11,
        ['M'] = 12, ['N'] = 13, ['O'] = 14, ['P'] = 15, ['Q'] = 16, ['R'] = 17,
        ['S'] = 18, ['T'] = 19, ['U'] = 20, ['V'] = 21, ['W'] = 22, ['X'] = 23,
        ['Y'] = 24, ['Z'] = 25,
        ['a'] = 26, ['b'] = 27, ['c'] = 28, ['d'] = 29, ['e'] = 30, ['f'] = 31,
        ['g'] = 32, ['h'] = 33, ['i'] = 34, ['j'] = 35, ['k'] = 36, ['l'] = 37,
        ['m'] = 38, ['n'] = 39, ['o'] = 40, ['p'] = 41, ['q'] = 42, ['r'] = 43,
        ['s'] = 44, ['t'] = 45, ['u'] = 46, ['v'] = 47, ['w'] = 48, ['x'] = 49,
        ['y'] = 50, ['z'] = 51,
        ['0'] = 52, ['1'] = 53, ['2'] = 54, ['3'] = 55, ['4'] = 56, ['5'] = 57,
        ['6'] = 58, ['7'] = 59, ['8'] = 60, ['9'] = 61, ['+'] = 62, ['/'] = 63
    };

    std::vector<uint8_t> result;
    result.reserve((encoded.size() / 4) * 3);

    uint32_t accumulator = 0;
    int bits = 0;

    for (char c : encoded) {
        if (c == '=') break;
        if (c < 0 || c > 255 || lookup[static_cast<uint8_t>(c)] == 0 && c != 'A') continue;

        accumulator = (accumulator << 6) | lookup[static_cast<uint8_t>(c)];
        bits += 6;

        if (bits >= 8) {
            bits -= 8;
            result.push_back(static_cast<uint8_t>((accumulator >> bits) & 0xFF));
        }
    }
    return result;
}

std::vector<uint8_t> CryptoService::xor_cipher(const std::vector<uint8_t>& data, const std::string& key) {
    std::vector<uint8_t> result(data.size());
    for (std::size_t i = 0; i < data.size(); ++i) {
        result[i] = data[i] ^ key[i % key.size()];
    }
    return result;
}

std::string CryptoService::hex_encode(const std::vector<uint8_t>& data) {
    static constexpr char hex_chars[] = "0123456789abcdef";
    std::string result;
    result.reserve(data.size() * 2);
    for (auto byte : data) {
        result += hex_chars[(byte >> 4) & 0x0F];
        result += hex_chars[byte & 0x0F];
    }
    return result;
}

} // namespace neo::security

#include <gtest/gtest.h>
#include <neo/memory/arena.hpp>

using namespace neo::memory;

TEST(ArenaTest, Creation) {
    Arena arena(1024);
    EXPECT_EQ(arena.capacity(), 1024u);
    EXPECT_EQ(arena.used(), 0u);
    EXPECT_EQ(arena.free(), 1024u);
}

TEST(ArenaTest, SimpleAllocation) {
    Arena arena(256);
    void* ptr = arena.allocate(64);
    EXPECT_NE(ptr, nullptr);
    EXPECT_EQ(arena.used(), 64u);
    EXPECT_EQ(arena.free(), 192u);
}

TEST(ArenaTest, MultipleAllocations) {
    Arena arena(256);
    void* p1 = arena.allocate(32);
    void* p2 = arena.allocate(64);
    void* p3 = arena.allocate(128);

    EXPECT_NE(p1, nullptr);
    EXPECT_NE(p2, nullptr);
    EXPECT_NE(p3, nullptr);
    EXPECT_LT(p1, p2);
    EXPECT_LT(p2, p3);
    EXPECT_EQ(arena.used(), 224u);
}

TEST(ArenaTest, AlignedAllocation) {
    Arena arena(256);
    void* p1 = arena.allocate(1, 64);
    EXPECT_EQ(reinterpret_cast<std::uintptr_t>(p1) % 64, 0u);

    void* p2 = arena.allocate(1, 128);
    EXPECT_EQ(reinterpret_cast<std::uintptr_t>(p2) % 128, 0u);
}

TEST(ArenaTest, Reset) {
    Arena arena(256);
    arena.allocate(128);
    EXPECT_EQ(arena.used(), 128u);

    arena.reset();
    EXPECT_EQ(arena.used(), 0u);
    EXPECT_EQ(arena.free(), 256u);
}

TEST(ArenaTest, AllocationExhaustion) {
    Arena arena(64);
    arena.allocate(32);
    arena.allocate(32);
    EXPECT_THROW(arena.allocate(1), neo::core::Error);
}

TEST(ArenaTest, Owns) {
    Arena arena(256);
    void* ptr = arena.allocate(64);
    EXPECT_TRUE(arena.owns(ptr));

    int stack_var = 42;
    EXPECT_FALSE(arena.owns(&stack_var));
}

TEST(ArenaTest, NoDoubleFree) {
    Arena arena(256);
    arena.allocate(128);
    arena.reset();
    arena.allocate(128);
    EXPECT_EQ(arena.used(), 128u);
}

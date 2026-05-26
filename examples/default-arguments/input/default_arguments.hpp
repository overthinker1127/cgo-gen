#pragma once

inline int Clamp(int value, int max = 100) {
    if (value > max) {
        return max;
    }
    return value;
}

class DefaultCounter {
public:
    explicit DefaultCounter(int start = 0)
        : value_(start) {}

    ~DefaultCounter() = default;

    int Value() const {
        return value_;
    }

    int Add(int value, int multiplier = 1) {
        value_ += value * multiplier;
        return value_;
    }

private:
    int value_;
};

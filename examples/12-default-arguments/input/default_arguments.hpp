#pragma once

inline int Clamp(int value, int min = 0, int max = 100) {
    if (value < min) {
        return min;
    }
    if (value > max) {
        return max;
    }
    return value;
}

class DefaultCounter {
public:
    explicit DefaultCounter(int start = 0, int step = 1)
        : value_(start), step_(step) {}

    ~DefaultCounter() = default;

    int Value() const {
        return value_;
    }

    int Add(int value, int multiplier = 1) {
        value_ += value * multiplier + step_;
        return value_;
    }

private:
    int value_;
    int step_;
};

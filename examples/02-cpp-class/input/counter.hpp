#pragma once

#include <string>

namespace metrics {

class Counter {
public:
    explicit Counter(int initial_value);
    ~Counter();

    int value() const;
    void increment(int amount);
    std::string label() const;

private:
    int value_;
};

int clamp_to_zero(int value);

} // namespace metrics

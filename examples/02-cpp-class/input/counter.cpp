#include "counter.hpp"

namespace metrics {

Counter::Counter(int initial_value) : value_(initial_value) {}

Counter::~Counter() = default;

int Counter::value() const {
    return value_;
}

void Counter::increment(int amount) {
    value_ += amount;
}

std::string Counter::label() const {
    return "requests";
}

int clamp_to_zero(int value) {
    return value < 0 ? 0 : value;
}

} // namespace metrics

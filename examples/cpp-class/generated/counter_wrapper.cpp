#include "counter_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "counter.hpp"

metricsCounterHandle* cgowrap_metrics_Counter_new(int initial_value) {
    return reinterpret_cast<metricsCounterHandle*>(new metrics::Counter(initial_value));
}

void cgowrap_metrics_Counter_delete(metricsCounterHandle* self) {
    delete reinterpret_cast<metrics::Counter*>(self);
}

int cgowrap_metrics_Counter_value(const metricsCounterHandle* self) {
    return reinterpret_cast<const metrics::Counter*>(self)->value();
}

void cgowrap_metrics_Counter_increment(metricsCounterHandle* self, int amount) {
    reinterpret_cast<metrics::Counter*>(self)->increment(amount);
}

char* cgowrap_metrics_Counter_label(const metricsCounterHandle* self) {
    std::string result = reinterpret_cast<const metrics::Counter*>(self)->label();
    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));
    if (buffer == nullptr) {
        return nullptr;
    }
    std::memcpy(buffer, result.c_str(), result.size() + 1);
    return buffer;
}

int cgowrap_metrics_clamp_to_zero(int value) {
    return metrics::clamp_to_zero(value);
}

void cgowrap_string_free(char* value) {
    free(value);
}

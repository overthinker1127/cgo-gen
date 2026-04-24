#include "ThingModel.hpp"

ThingModel::ThingModel() : value_(0), name_("default") {}

ThingModel::~ThingModel() = default;

int32_t ThingModel::GetValue() const { return value_; }

void ThingModel::SetValue(int32_t value) { value_ = value; }

const char* ThingModel::GetName() const { return name_.c_str(); }

void ThingModel::SetName(const char* name) {
    if (name == nullptr) {
        name_.clear();
        return;
    }
    name_ = name;
}

#pragma once

#include <stdint.h>
#include <string>

class ThingModel {
public:
    ThingModel();
    ~ThingModel();

    int32_t GetValue() const;
    void SetValue(int32_t value);

    const char* GetName() const;
    void SetName(const char* name);

private:
    int32_t value_;
    std::string name_;
};

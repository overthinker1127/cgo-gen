#pragma once

#include <stdint.h>
#include <string>

class InventoryItem {
public:
    InventoryItem();
    ~InventoryItem();

    int32_t Id() const;
    void SetId(int32_t id);

    int32_t Quantity() const;
    void SetQuantity(int32_t quantity);

    const char* Name() const;
    void SetName(const char* name);

private:
    int32_t id_;
    int32_t quantity_;
    std::string name_;
};

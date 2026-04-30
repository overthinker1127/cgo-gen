#pragma once

#include <stdint.h>

#include "inventory_item.hpp"

class InventoryService {
public:
    InventoryService();
    ~InventoryService();

    bool LoadItem(int32_t id, InventoryItem& out) const;
    bool NextItem(int32_t& cursor, InventoryItem& out) const;
};

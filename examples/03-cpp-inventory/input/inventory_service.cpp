#include "inventory_service.hpp"

#include <string>

InventoryService::InventoryService() = default;

InventoryService::~InventoryService() = default;

bool InventoryService::LoadItem(int32_t id, InventoryItem& out) const {
    if (id <= 0) {
        return false;
    }

    out.SetId(id);
    out.SetQuantity(id * 10);
    out.SetName(("sku-" + std::to_string(id)).c_str());
    return true;
}

bool InventoryService::NextItem(int32_t& cursor, InventoryItem& out) const {
    cursor += 1;
    out.SetId(cursor);
    out.SetQuantity(cursor * 100);
    out.SetName(("next-sku-" + std::to_string(cursor)).c_str());
    return true;
}

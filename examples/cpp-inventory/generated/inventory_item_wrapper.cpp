#include "inventory_item_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "inventory_item.hpp"

InventoryItemHandle* cgowrap_InventoryItem_new(void) {
    return reinterpret_cast<InventoryItemHandle*>(new InventoryItem());
}

void cgowrap_InventoryItem_delete(InventoryItemHandle* self) {
    delete reinterpret_cast<InventoryItem*>(self);
}

int cgowrap_InventoryItem_Id(const InventoryItemHandle* self) {
    return reinterpret_cast<const InventoryItem*>(self)->Id();
}

void cgowrap_InventoryItem_SetId(InventoryItemHandle* self, int id) {
    reinterpret_cast<InventoryItem*>(self)->SetId(static_cast<int32_t>(id));
}

int cgowrap_InventoryItem_Quantity(const InventoryItemHandle* self) {
    return reinterpret_cast<const InventoryItem*>(self)->Quantity();
}

void cgowrap_InventoryItem_SetQuantity(InventoryItemHandle* self, int quantity) {
    reinterpret_cast<InventoryItem*>(self)->SetQuantity(static_cast<int32_t>(quantity));
}

const char* cgowrap_InventoryItem_Name(const InventoryItemHandle* self) {
    return reinterpret_cast<const InventoryItem*>(self)->Name();
}

void cgowrap_InventoryItem_SetName(InventoryItemHandle* self, const char* name) {
    reinterpret_cast<InventoryItem*>(self)->SetName(name);
}

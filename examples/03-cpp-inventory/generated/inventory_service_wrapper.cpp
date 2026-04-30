#include "inventory_service_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "inventory_item.hpp"
#include "inventory_service.hpp"

InventoryServiceHandle* cgowrap_InventoryService_new(void) {
    return reinterpret_cast<InventoryServiceHandle*>(new InventoryService());
}

void cgowrap_InventoryService_delete(InventoryServiceHandle* self) {
    delete reinterpret_cast<InventoryService*>(self);
}

bool cgowrap_InventoryService_LoadItem(const InventoryServiceHandle* self, int id, InventoryItemHandle* out) {
    return reinterpret_cast<const InventoryService*>(self)->LoadItem(static_cast<int32_t>(id), *reinterpret_cast<InventoryItem*>(out));
}

bool cgowrap_InventoryService_NextItem(const InventoryServiceHandle* self, int32_t* cursor, InventoryItemHandle* out) {
    return reinterpret_cast<const InventoryService*>(self)->NextItem(*cursor, *reinterpret_cast<InventoryItem*>(out));
}

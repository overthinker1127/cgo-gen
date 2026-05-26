#ifndef CGOWRAP_INVENTORY_SERVICE_WRAPPER_H
#define CGOWRAP_INVENTORY_SERVICE_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct InventoryServiceHandle InventoryServiceHandle;
typedef struct InventoryItemHandle InventoryItemHandle;

InventoryServiceHandle* cgowrap_InventoryService_new(void);
void cgowrap_InventoryService_delete(InventoryServiceHandle* self);
bool cgowrap_InventoryService_LoadItem(const InventoryServiceHandle* self, int id, InventoryItemHandle* out);
bool cgowrap_InventoryService_NextItem(const InventoryServiceHandle* self, int32_t* cursor, InventoryItemHandle* out);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_INVENTORY_SERVICE_WRAPPER_H */

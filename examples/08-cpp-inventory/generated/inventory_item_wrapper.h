#ifndef CGOWRAP_INVENTORY_ITEM_WRAPPER_H
#define CGOWRAP_INVENTORY_ITEM_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct InventoryItemHandle InventoryItemHandle;

InventoryItemHandle* cgowrap_InventoryItem_new(void);
void cgowrap_InventoryItem_delete(InventoryItemHandle* self);
int cgowrap_InventoryItem_Id(const InventoryItemHandle* self);
void cgowrap_InventoryItem_SetId(InventoryItemHandle* self, int id);
int cgowrap_InventoryItem_Quantity(const InventoryItemHandle* self);
void cgowrap_InventoryItem_SetQuantity(InventoryItemHandle* self, int quantity);
const char* cgowrap_InventoryItem_Name(const InventoryItemHandle* self);
void cgowrap_InventoryItem_SetName(InventoryItemHandle* self, const char* name);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_INVENTORY_ITEM_WRAPPER_H */

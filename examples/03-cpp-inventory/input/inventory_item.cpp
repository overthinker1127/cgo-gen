#include "inventory_item.hpp"

InventoryItem::InventoryItem() : id_(0), quantity_(0), name_("") {}

InventoryItem::~InventoryItem() = default;

int32_t InventoryItem::Id() const {
    return id_;
}

void InventoryItem::SetId(int32_t id) {
    id_ = id;
}

int32_t InventoryItem::Quantity() const {
    return quantity_;
}

void InventoryItem::SetQuantity(int32_t quantity) {
    quantity_ = quantity;
}

const char* InventoryItem::Name() const {
    return name_.c_str();
}

void InventoryItem::SetName(const char* name) {
    name_ = name == nullptr ? "" : name;
}

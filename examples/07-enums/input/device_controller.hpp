#pragma once

typedef enum {
    DeviceStateOffline = 0,
    DeviceStateOnline = 1,
    DeviceStateError = 2,
} DeviceState;

enum DeviceMode {
    DeviceModeManual = 10,
    DeviceModeAutomatic = 20,
};

enum {
    DeviceFeatureLogging = 1,
    DeviceFeatureMetrics = 2,
};

class DeviceController {
public:
    DeviceController()
        : state_(DeviceStateOffline), mode_(DeviceModeManual), feature_mask_(0) {}
    ~DeviceController() = default;

    DeviceState State() const {
        return state_;
    }

    void SetState(DeviceState state) {
        state_ = state;
    }

    DeviceMode Mode() const {
        return mode_;
    }

    bool SetMode(DeviceMode mode) {
        mode_ = mode;
        return mode_ == DeviceModeAutomatic;
    }

    void EnableFeature(int feature) {
        feature_mask_ |= feature;
    }

    bool IsFeatureEnabled(int feature) const {
        return (feature_mask_ & feature) != 0;
    }

private:
    DeviceState state_;
    DeviceMode mode_;
    int feature_mask_;
};

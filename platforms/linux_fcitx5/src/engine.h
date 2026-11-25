#ifndef VIETFLOW_FCITX5_ENGINE_H
#define VIETFLOW_FCITX5_ENGINE_H

#include "config.h"
#include "vietflow_core.h"

#include <fcitx-config/iniparser.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>

namespace vietflow {

class VietFlowState final : public fcitx::InputContextProperty {
public:
    VietFlowState();
    ~VietFlowState() override;

    VietFlowState(const VietFlowState &) = delete;
    VietFlowState &operator=(const VietFlowState &) = delete;

    VfEngine *engine() const { return engine_; }
    void syncConfig(const VietFlowConfig &config);
    void resetEngine();

    bool vietnameseMode() const { return vietnameseMode_; }
    void setVietnameseMode(bool enabled) { vietnameseMode_ = enabled; }
    std::string &enBuffer() { return enBuffer_; }
    const std::string &enBuffer() const { return enBuffer_; }
    void clearEnBuffer() { enBuffer_.clear(); }

private:
    VfEngine *engine_ = nullptr;
    bool vietnameseMode_ = true;
    std::string enBuffer_;
};

class VietFlowEngine final : public fcitx::InputMethodEngineV2 {
public:
    explicit VietFlowEngine(fcitx::Instance *instance);
    ~VietFlowEngine() override = default;

    void keyEvent(const fcitx::InputMethodEntry &entry,
                  fcitx::KeyEvent &event) override;
    void activate(const fcitx::InputMethodEntry &entry,
                  fcitx::InputContextEvent &event) override;
    void deactivate(const fcitx::InputMethodEntry &entry,
                    fcitx::InputContextEvent &event) override;
    void reset(const fcitx::InputMethodEntry &entry,
               fcitx::InputContextEvent &event) override;

    const fcitx::Configuration *getConfig() const override { return &config_; }
    void setConfig(const fcitx::RawConfig &rawConfig) override;
    void reloadConfig() override;

    std::string subModeLabelImpl(const fcitx::InputMethodEntry &entry,
                                 fcitx::InputContext &ic) override;

private:
    void clearPreedit(fcitx::InputContext *ic, VietFlowState *state);
    void updatePreeditText(fcitx::InputContext *ic, const std::string &utf8);
    void commitCurrentPreedit(fcitx::InputContext *ic, VietFlowState *state);
    void commitWithBreak(fcitx::InputContext *ic, VietFlowState *state,
                         uint32_t breakChar);
    void handleBackspace(fcitx::InputContext *ic, VietFlowState *state,
                         fcitx::KeyEvent &event);
    void applyAutoSwitchRequest(fcitx::InputContext *ic, VietFlowState *state);
    bool isAppExcluded(const fcitx::InputContext *ic) const;
    void handleEnglishModeKey(fcitx::InputContext *ic, VietFlowState *state,
                              fcitx::KeyEvent &event);
    void handleEnglishModeBreak(fcitx::InputContext *ic, VietFlowState *state,
                                uint32_t breakChar, fcitx::KeyEvent &event,
                                bool consumeOnCommit);

    fcitx::Instance *instance_;
    VietFlowConfig config_;
    fcitx::FactoryFor<VietFlowState> factory_;
};

class VietFlowFactory final : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        return new VietFlowEngine(manager->instance());
    }
};

} // namespace vietflow

#endif // VIETFLOW_FCITX5_ENGINE_H

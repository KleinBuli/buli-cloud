package dev.bulicloud.bridge.model;

import dev.bulicloud.bridge.model.enums.InstanceMode;

public record CloudInstance (String id, String groupName, String template_name, InstanceMode instanceMode) { }

package dev.bulicloud.bridge.model;

import dev.bulicloud.bridge.model.enums.GroupRole;

import java.util.List;

public record CloudGroup(String id, String groupName, GroupRole groupRole, List<String> template_names, boolean maintenance) {  }
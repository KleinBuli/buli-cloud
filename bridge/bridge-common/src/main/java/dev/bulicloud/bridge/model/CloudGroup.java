package dev.bulicloud.bridge.model;

import dev.bulicloud.bridge.model.enums.GroupRole;

import java.util.List;

@com.fasterxml.jackson.annotation.JsonIgnoreProperties(ignoreUnknown = true)
public record CloudGroup(String id, @com.fasterxml.jackson.annotation.JsonProperty("name") String groupName, @com.fasterxml.jackson.annotation.JsonProperty("role") GroupRole groupRole, List<String> template_names, boolean maintenance) {  }
package dev.bulicloud.bridge.model.enums;

public enum GroupRole {
    @com.fasterxml.jackson.annotation.JsonProperty("Proxy") PROXY,
    @com.fasterxml.jackson.annotation.JsonProperty("ProxiedServer") PROXIED_SERVER,
    @com.fasterxml.jackson.annotation.JsonProperty("StandaloneServer") STANDALONE_SERVER,
}

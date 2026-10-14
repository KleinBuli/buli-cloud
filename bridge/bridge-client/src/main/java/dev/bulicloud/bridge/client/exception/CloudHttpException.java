package dev.bulicloud.bridge.client.exception;

public class CloudHttpException extends RuntimeException {

    private final int statusCode;

    public CloudHttpException(int statusCode, String message) {

        super(message);
        this.statusCode = statusCode;
    }
}

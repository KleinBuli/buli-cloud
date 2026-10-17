plugins {
    id("java")
}

group = "dev.bulicloud"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(platform("org.junit:junit-bom:5.10.0"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
subprojects {
    plugins.withId("java") {
        extensions.configure<JavaPluginExtension> {
            toolchain.languageVersion.set(JavaLanguageVersion.of(25))
        }
    }
}

val stageBridges by tasks.registering(Copy::class) {
    dependsOn(":bridge-velocity:jar")
    from(project(":bridge-velocity").layout.buildDirectory.file("libs/bulicloud-bridge-velocity.jar"))
    into(layout.projectDirectory.dir("../dist/data/bridge"))
}

tasks.named("assemble") {
    dependsOn(stageBridges)
}

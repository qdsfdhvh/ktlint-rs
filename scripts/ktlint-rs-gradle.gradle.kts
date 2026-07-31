import org.gradle.api.DefaultTask
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Optional

abstract class KtlintRsTask : DefaultTask() {
    @get:Input
    @get:Optional
    var autoCorrect: Boolean = false

    @TaskAction
    fun lint() {
        val binary = findBinary()
        val targets = project.fileTree(project.projectDir) {
            include("**/src/**/*.kt")
            exclude("**/generated/**")
            exclude("**/build/**")
        }
        val args = mutableListOf(binary.absolutePath)
        if (autoCorrect) args.add("--format")
        targets.files.forEach { args.add(it.absolutePath) }
        val process = ProcessBuilder(args)
            .inheritIO()
            .start()
        val exitCode = process.waitFor()
        if (exitCode != 0 && !autoCorrect) {
            throw GradleException("ktlint-rs found violations. Run with autoCorrect=true to fix.")
        }
    }

    private fun findBinary(): File {
        // Check common locations
        val candidates = listOf(
            "ktlint-rs",
            System.getProperty("user.home") + "/.cargo/bin/ktlint-rs",
            project.rootProject.file("tools/ktlint-rs").absolutePath,
        )
        candidates.forEach { name ->
            val f = File(name)
            if (f.exists() && f.canExecute()) return f
        }
        // Try PATH
        return File("ktlint-rs")
    }
}

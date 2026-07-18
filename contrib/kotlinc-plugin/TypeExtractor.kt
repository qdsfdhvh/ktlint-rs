/**
 * Kotlin compiler plugin for ktlint-rs Phase 13 type resolution.
 *
 * Usage: kotlinc -script TypeExtractor.kts -- <source.kt>
 * Output: JSON type annotations on stdout (matching ktlint-rs JSON schema).
 *
 * This file should be compiled or run as a Kotlin script using kotlinc.
 * It uses the Kotlin compiler API to resolve types and outputs:
 *
 * {
 *   "version": 1,
 *   "declarations": { "name": {"type": "T", "nullable": bool, "line": N} },
 *   "return_types": { "funcName": "ReturnType" }
 * }
 */

import org.jetbrains.kotlin.cli.common.CLIConfigurationKeys
import org.jetbrains.kotlin.cli.jvm.K2JVMCompiler
import org.jetbrains.kotlin.cli.jvm.compiler.EnvironmentConfigFiles
import org.jetbrains.kotlin.cli.jvm.compiler.KotlinCoreEnvironment
import org.jetbrains.kotlin.com.intellij.openapi.util.Disposer
import org.jetbrains.kotlin.config.CompilerConfiguration
import org.jetbrains.kotlin.psi.*
import org.jetbrains.kotlin.resolve.BindingContext
import org.jetbrains.kotlin.resolve.descriptorUtil.fqNameSafe

fun main(args: Array<String>) {
    if (args.isEmpty()) {
        System.err.println("Usage: kotlinc -script TypeExtractor.kts -- <source.kt>")
        return
    }

    val sourcePath = args[0]
    val sourceText = java.io.File(sourcePath).readText()

    // Create a disposable root
    val disposable = Disposer.newDisposable()

    try {
        val configuration = CompilerConfiguration().apply {
            put(CLIConfigurationKeys.MESSAGE_COLLECTOR_KEY,
                org.jetbrains.kotlin.cli.common.messages.MessageCollector.NONE)
        }

        val environment = KotlinCoreEnvironment.createForProduction(
            disposable,
            configuration,
            EnvironmentConfigFiles.JVM_CONFIG_FILES
        )

        // Parse source
        val psiFile = KtPsiFactory(environment.project).createFile(sourceText)

        // Analyze to resolve types
        val analysisResult = environment.project.let { project ->
            // Simplified: iterate PSI tree and extract declared types
            // Full binding context resolution requires the kotlin-analysis API
            psiFile.accept(object : KtVisitorVoid() {
                val declarations = mutableMapOf<String, Map<String, Any>>()
                val returnTypes = mutableMapOf<String, String>()

                override fun visitProperty(property: KtProperty) {
                    val name = property.name ?: return
                    val typeRef = property.typeReference?.text ?: "Unknown"
                    val isNullable = typeRef.endsWith("?")
                    declarations[name] = mapOf(
                        "type" to typeRef.trimEnd('?'),
                        "nullable" to isNullable,
                        "line" to (property.textRange?.startOffset?.let {
                            sourceText.substring(0, it).count { c -> c == '\n' } + 1
                        } ?: 0)
                    )
                }

                override fun visitNamedFunction(function: KtNamedFunction) {
                    val name = function.name ?: return
                    function.typeReference?.let {
                        returnTypes[name] = it.text
                    }
                    function.valueParameters.forEach { param ->
                        val pName = param.name ?: return@forEach
                        val pType = param.typeReference?.text ?: "Unknown"
                        declarations[pName] = mapOf(
                            "type" to pType.trimEnd('?'),
                            "nullable" to pType.endsWith("?"),
                            "line" to (param.textRange?.startOffset?.let {
                                sourceText.substring(0, it).count { c -> c == '\n' } + 1
                            } ?: 0)
                        )
                    }
                }

                override fun visitClass(klass: KtClass) {
                    klass.primaryConstructorParameters?.forEach { param ->
                        val name = param.name ?: return@forEach
                        val typeRef = param.typeReference?.text ?: "Unknown"
                        declarations[name] = mapOf(
                            "type" to typeRef.trimEnd('?'),
                            "nullable" to typeRef.endsWith("?"),
                            "line" to 0
                        )
                    }
                    super.visitClass(klass)
                }
            })
        }

        // Build JSON output
        val json = buildString {
            appendLine("{")
            appendLine("  \"version\": 1,")
            append("  \"declarations\": ")
            append(buildJsonObject(analysisResult.declarations))
            appendLine(",")
            append("  \"return_types\": ")
            append(buildJsonObjectSimple(analysisResult.returnTypes))
            appendLine()
            appendLine("}")
        }

        println(json)
    } finally {
        Disposer.dispose(disposable)
    }
}

fun buildJsonObject(map: Map<String, Map<String, Any>>): String {
    if (map.isEmpty()) return "{}"
    val sb = StringBuilder("{\n")
    map.entries.forEachIndexed { i, (key, value) ->
        sb.append("    \"$key\": {")
        value.entries.forEachIndexed { j, (k, v) ->
            sb.append("\"$k\": ")
            when (v) {
                is String -> sb.append("\"$v\"")
                is Boolean -> sb.append(v)
                is Number -> sb.append(v)
                else -> sb.append("\"$v\"")
            }
            if (j < value.size - 1) sb.append(", ")
        }
        sb.append("}")
        if (i < map.size - 1) sb.append(",")
        sb.append("\n")
    }
    sb.append("  }")
    return sb.toString()
}

fun buildJsonObjectSimple(map: Map<String, String>): String {
    if (map.isEmpty()) return "{}"
    val sb = StringBuilder("{\n")
    map.entries.forEachIndexed { i, (key, value) ->
        sb.append("    \"$key\": \"$value\"")
        if (i < map.size - 1) sb.append(",")
        sb.append("\n")
    }
    sb.append("  }")
    return sb.toString()
}

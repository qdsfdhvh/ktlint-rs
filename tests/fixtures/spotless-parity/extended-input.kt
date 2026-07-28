package parity

annotation class Ann

@Ann class Annotated(val first:Int,val second:String)

fun branching(value:Int):String = when(value){
1->"one"
else->"other"
}

fun guarded(value:Int):Int = try{
require(value>0)
value
}catch(exception:IllegalArgumentException){
0
}

fun chaining():String = "value".trim().lowercase().replace("v","V")

fun nullable(value:String?):String?=value?.trim()?.takeIf{it.isNotEmpty()}

val callback:(value:String)->Unit={value->println(value)}

val collection=mapOf("one" to 1,"two" to 2,)

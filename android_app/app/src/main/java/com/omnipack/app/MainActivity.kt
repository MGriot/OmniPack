package com.omnipack.app

import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    companion object {
        init {
            System.loadLibrary("omnipack_jni")
        }
    }

    private external fun packSample(): String

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val resultText = findViewById<TextView>(R.id.result_text)
        findViewById<Button>(R.id.pack_button).setOnClickListener {
            resultText.text = packSample()
        }
    }
}

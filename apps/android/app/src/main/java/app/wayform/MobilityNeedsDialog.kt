package app.wayform

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

@Composable
fun MobilityNeedsDialog(current: MobilityNeeds, onSave: (MobilityNeeds) -> Unit, onDismiss: () -> Unit) {
    var draft by remember { mutableStateOf(current) }
    AlertDialog(onDismissRequest = onDismiss, title = { Text("Mobility requirements") }, text = {
        Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("These are hard requirements. A route needs surveyed evidence to satisfy them. Leave an optional measurement blank to make no requirement for it.")
            OutlinedTextField(draft.widthCm, { if (it.length <= 8) draft = draft.copy(widthCm = it) }, label = { Text("Minimum clear width (cm)") }, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), singleLine = true)
            OutlinedTextField(draft.slopePercent, { if (it.length <= 8) draft = draft.copy(slopePercent = it) }, label = { Text("Maximum slope (%)") }, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), singleLine = true)
            Row {
                Checkbox(draft.noSteps, { draft = draft.copy(noSteps = it) }, modifier = Modifier.semantics { contentDescription = "Require surveyed zero steps" })
                Text("Require surveyed zero steps", modifier = Modifier.padding(top = 12.dp))
            }
            draft.validationError()?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            Text("The bundled sample venues contain authored flags, not surveyed measurements. Requirements will remain unknown until appropriate evidence is available.", style = MaterialTheme.typography.bodySmall)
        }
    }, confirmButton = { TextButton(onClick = { onSave(draft) }, enabled = draft.validationError() == null) { Text("Save requirements") } }, dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } })
}

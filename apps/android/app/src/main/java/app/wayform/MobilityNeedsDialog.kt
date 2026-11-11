package app.wayform

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

@Composable
fun MobilityNeedsDialog(current: MobilityNeeds, onSave: (MobilityNeeds) -> Unit, onDismiss: () -> Unit) {
    var draft by remember { mutableStateOf(current) }
    AlertDialog(onDismissRequest = onDismiss, title = { Text("Mobility requirements") }, text = {
        Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Choose what your route must meet. Leave optional measurements blank.")
            OutlinedTextField(draft.widthCm, { if (it.length <= 8) draft = draft.copy(widthCm = it) }, label = { Text("Minimum clear width (cm)") }, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), singleLine = true)
            OutlinedTextField(draft.slopePercent, { if (it.length <= 8) draft = draft.copy(slopePercent = it) }, label = { Text("Maximum slope (%)") }, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), singleLine = true)
            Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).toggleable(value = draft.noSteps,
                role = Role.Checkbox, onValueChange = { draft = draft.copy(noSteps = it) }), verticalAlignment = Alignment.CenterVertically) {
                Checkbox(draft.noSteps, onCheckedChange = null)
                Text("Require surveyed zero steps", modifier = Modifier.padding(start = 12.dp))
            }
            draft.validationError()?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
    }, confirmButton = { TextButton(onClick = { onSave(draft) }, enabled = draft.validationError() == null) { Text("Save requirements") } }, dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } })
}

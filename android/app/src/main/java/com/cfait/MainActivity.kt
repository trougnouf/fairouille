// File: android/app/src/main/java/com/cfait/MainActivity.kt
package com.cfait

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.cfait.core.CfaitMobile
import com.cfait.core.MobileTask
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val api = CfaitMobile(filesDir.absolutePath)
        setContent {
            val darkTheme = isSystemInDarkTheme()
            val colors = if (darkTheme) darkColorScheme() else lightColorScheme()
            
            MaterialTheme(colorScheme = colors) {
                CfaitNavHost(api)
            }
        }
    }
}

// --- NAVIGATION & STATE ---

@Composable
fun CfaitNavHost(api: CfaitMobile) {
    val navController = rememberNavController()
    var tasks by remember { mutableStateOf<List<MobileTask>>(emptyList()) }
    var hideCompleted by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    var isLoading by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    fun refresh() {
        scope.launch {
            isLoading = true
            try {
                try { api.loadAndConnect() } catch (_: Exception) {}
                hideCompleted = api.getConfig().hideCompleted
                tasks = api.getTasks()
            } catch (e: Exception) {
                statusMessage = "Error: ${e.message}"
            } finally {
                isLoading = false
            }
        }
    }

    LaunchedEffect(Unit) { refresh() }

    NavHost(navController, startDestination = "home") {
        composable("home") {
            HomeScreen(
                tasks = tasks,
                hideCompleted = hideCompleted,
                isLoading = isLoading,
                onRefresh = { refresh() },
                onAddTask = { txt ->
                    scope.launch {
                        try {
                            api.addTaskSmart(txt)
                            refresh()
                        } catch (e: Exception) { statusMessage = e.message }
                    }
                },
                onToggle = { uid ->
                    scope.launch {
                        try {
                            api.toggleTask(uid)
                            refresh()
                        } catch(e: Exception) { statusMessage = e.message }
                    }
                },
                onDelete = { uid ->
                    scope.launch {
                        try {
                            api.deleteTask(uid)
                            refresh()
                        } catch (e: Exception) { statusMessage = e.message }
                    }
                },
                onSettings = { navController.navigate("settings") }
            )
        }
        composable("settings") {
            SettingsScreen(
                api = api,
                onBack = {
                    navController.popBackStack()
                    refresh()
                }
            )
        }
    }
}

// --- SCREENS ---

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    tasks: List<MobileTask>,
    hideCompleted: Boolean,
    isLoading: Boolean,
    onRefresh: () -> Unit,
    onAddTask: (String) -> Unit,
    onToggle: (String) -> Unit,
    onDelete: (String) -> Unit,
    onSettings: () -> Unit
) {
    var filterTag by remember { mutableStateOf<String?>(null) }
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    var newTaskText by remember { mutableStateOf("") }

    val allTags = tasks.flatMap { it.categories }.distinct().sorted()

    // --- SORTING FIX APPLIED HERE ---
    val displayTasks = tasks
        .filter { task ->
            (filterTag == null || task.categories.contains(filterTag)) &&
            (!hideCompleted || !task.isDone)
        }
        .sortedWith(
            compareBy(
                { it.isDone },
                // FIX: Cast .priority to Int explicitly so types match 10
                { if (it.priority == 0.toUByte()) 10 else it.priority.toInt() }
            )
        )
    // --------------------------------

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet {
                Spacer(Modifier.height(12.dp))
                NavigationDrawerItem(
                    label = { Text("All Tasks") },
                    selected = filterTag == null,
                    onClick = { filterTag = null; scope.launch { drawerState.close() } },
                    icon = { Icon(Icons.Default.List, null) },
                    modifier = Modifier.padding(NavigationDrawerItemDefaults.ItemPadding)
                )
                Divider(Modifier.padding(vertical = 8.dp))
                Text("Tags", modifier = Modifier.padding(16.dp), fontWeight = FontWeight.Bold)
                allTags.forEach { tag ->
                    NavigationDrawerItem(
                        label = { Text("#$tag") },
                        selected = filterTag == tag,
                        onClick = { filterTag = tag; scope.launch { drawerState.close() } },
                        icon = { Icon(Icons.Default.Label, null) },
                        modifier = Modifier.padding(NavigationDrawerItemDefaults.ItemPadding)
                    )
                }
            }
        }
    ) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text(if (filterTag == null) "Cfait" else "#$filterTag") },
                    navigationIcon = {
                        IconButton(onClick = { scope.launch { drawerState.open() } }) {
                            Icon(Icons.Default.Menu, "Menu")
                        }
                    },
                    actions = {
                        if (isLoading) {
                            CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                        } else {
                            IconButton(onClick = onRefresh) { Icon(Icons.Default.Refresh, "Sync") }
                        }
                        IconButton(onClick = onSettings) { Icon(Icons.Default.Settings, "Settings") }
                    }
                )
            },
            bottomBar = {
                Surface(tonalElevation = 3.dp) {
                    Row(
                        modifier = Modifier.padding(16.dp).navigationBarsPadding(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        OutlinedTextField(
                            value = newTaskText,
                            onValueChange = { newTaskText = it },
                            placeholder = { Text("!1 @tomorrow Buy milk") },
                            modifier = Modifier.weight(1f),
                            singleLine = true
                        )
                        Spacer(Modifier.width(8.dp))
                        Button(onClick = {
                            if (newTaskText.isNotBlank()) {
                                onAddTask(newTaskText)
                                newTaskText = ""
                            }
                        }) {
                            Icon(Icons.Default.Send, "Add")
                        }
                    }
                }
            }
        ) { padding ->
            LazyColumn(
                modifier = Modifier.padding(padding).fillMaxSize(),
                contentPadding = PaddingValues(bottom = 80.dp)
            ) {
                items(displayTasks, key = { it.uid }) { task ->
                    TaskRow(task, onToggle, onDelete)
                }
            }
        }
    }
}

@Composable
fun TaskRow(task: MobileTask, onToggle: (String) -> Unit, onDelete: (String) -> Unit) {
    val prioColor = getPriorityColor(task.priority.toInt())

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 4.dp),
        border = BorderStroke(1.dp, if (task.isDone) Color.Gray else prioColor),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Checkbox(checked = task.isDone, onCheckedChange = { onToggle(task.uid) })

            Column(modifier = Modifier.weight(1f).padding(horizontal = 8.dp)) {
                Text(
                    text = task.summary,
                    style = MaterialTheme.typography.bodyLarge,
                    color = if (task.isDone) Color.Gray else MaterialTheme.colorScheme.onSurface,
                    textDecoration = if (task.isDone) androidx.compose.ui.text.style.TextDecoration.LineThrough else null
                )

                Row(modifier = Modifier.padding(top = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (task.priority > 0.toUByte()) {
                        Text(
                            text = "!${task.priority}",
                            color = prioColor,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            // FIX: use end instead of right for RTL support
                            modifier = Modifier.padding(end = 8.dp)
                        )
                    }
                    if (!task.dueDateIso.isNullOrEmpty()) {
                        Icon(Icons.Default.CalendarToday, null, modifier = Modifier.size(12.dp), tint = Color.Gray)
                        Text(
                            text = task.dueDateIso!!.take(10),
                            fontSize = 12.sp,
                            color = Color.Gray,
                            // FIX: use end instead of right
                            modifier = Modifier.padding(start = 2.dp, end = 8.dp)
                        )
                    }
                    task.categories.forEach { tag ->
                        Surface(
                            color = MaterialTheme.colorScheme.secondaryContainer,
                            shape = RoundedCornerShape(4.dp),
                            modifier = Modifier.padding(end = 4.dp)
                        ) {
                            Text(
                                text = "#$tag",
                                fontSize = 10.sp,
                                modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp),
                                color = MaterialTheme.colorScheme.onSecondaryContainer
                            )
                        }
                    }
                }
            }

            IconButton(onClick = { onDelete(task.uid) }) {
                Icon(Icons.Default.Delete, "Delete", tint = MaterialTheme.colorScheme.error.copy(alpha = 0.5f))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(api: CfaitMobile, onBack: () -> Unit) {
    var url by remember { mutableStateOf("") }
    var user by remember { mutableStateOf("") }
    var pass by remember { mutableStateOf("") }
    var insecure by remember { mutableStateOf(false) }
    var hideCompleted by remember { mutableStateOf(false) }
    var status by remember { mutableStateOf("") }
    val scope = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        val cfg = api.getConfig()
        url = cfg.url
        user = cfg.username
        insecure = cfg.allowInsecure
        hideCompleted = cfg.hideCompleted
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, "Back") } }
            )
        }
    ) { p ->
        Column(modifier = Modifier.padding(p).padding(16.dp)) {
            OutlinedTextField(value = url, onValueChange = { url = it }, label = { Text("CalDAV URL") }, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            OutlinedTextField(value = user, onValueChange = { user = it }, label = { Text("Username") }, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            OutlinedTextField(
                value = pass,
                onValueChange = { pass = it },
                label = { Text("Password (Leave empty to keep)") },
                visualTransformation = PasswordVisualTransformation(),
                modifier = Modifier.fillMaxWidth()
            )
            Spacer(Modifier.height(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = insecure, onCheckedChange = { insecure = it })
                Text("Allow Insecure SSL")
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = hideCompleted, onCheckedChange = { hideCompleted = it })
                Text("Hide Completed Tasks")
            }
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = {
                    scope.launch {
                        status = "Saving..."
                        try {
                            api.saveConfig(url, user, pass, insecure, hideCompleted)
                            status = api.connect(url, user, pass, insecure)
                        } catch (e: Exception) {
                            status = "Error: ${e.message}"
                        }
                    }
                },
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Save & Connect")
            }
            Spacer(Modifier.height(16.dp))
            Text(status, color = if (status.startsWith("Error")) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary)
        }
    }
}

fun getPriorityColor(prio: Int): Color {
    return when (prio) {
        1 -> Color(0xFFFF4444)
        2 -> Color(0xFFFF8800)
        3 -> Color(0xFFFFBB33)
        4 -> Color(0xFFFFD700)
        5 -> Color(0xFFFFFF00)
        else -> Color.LightGray
    }
}
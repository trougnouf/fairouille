// File: android/app/src/main/java/com/cfait/MainActivity.kt
package com.cfait

import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.cfait.core.CfaitMobile
import com.cfait.core.MobileCalendar
import com.cfait.core.MobileTask
import com.cfait.core.MobileTag
import kotlinx.coroutines.launch

// --- FONTS & ICONS ---
val NerdFont = FontFamily(Font(R.font.symbols_nerd_font))

object NfIcons {
    fun get(code: Int): String = String(Character.toChars(code))
    val SEARCH = get(0xf002)
    val CALENDAR = get(0xf073)
    val TAG = get(0xf02b)
    val REFRESH = get(0xf021) 
    val SETTINGS = get(0xe690)
    val DELETE = get(0xf1f8)
    val CHECK = get(0xf00c)
    val CROSS = get(0xf00d)
    val PLAY = get(0xf04b)
    val PAUSE = get(0xf04c)
    val REPEAT = get(0xf0b6)
    val VISIBLE = get(0xea70)
    val HIDDEN = get(0xeae7)
    val WRITE_TARGET = get(0xf0cfb)
    val MENU = get(0xf0c9)
    val ADD = get(0xf067)
    val BACK = get(0xf060)
    val BLOCK = get(0xf479)
    val DOTS_CIRCLE = get(0xf1978) 
    val PRIORITY_UP = get(0xf0603)
    val PRIORITY_DOWN = get(0xf0604)
    val COPY = get(0xf0c5) 
    val EDIT = get(0xf040)
    val ARROW_RIGHT = get(0xf061)
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val api = CfaitMobile(filesDir.absolutePath)
        setContent {
            MaterialTheme(colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme()) {
                CfaitNavHost(api)
            }
        }
    }
}

@Composable
fun CfaitNavHost(api: CfaitMobile) {
    val navController = rememberNavController()
    var calendars by remember { mutableStateOf<List<MobileCalendar>>(emptyList()) }
    var tags by remember { mutableStateOf<List<MobileTag>>(emptyList()) }
    var defaultCalHref by remember { mutableStateOf<String?>(null) }
    
    val scope = rememberCoroutineScope()
    var isLoading by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    fun fastStart() {
        api.loadFromCache()
        calendars = api.getCalendars()
        scope.launch { tags = api.getAllTags() }
        defaultCalHref = api.getConfig().defaultCalendar
        scope.launch {
            isLoading = true
            try { 
                api.sync()
                calendars = api.getCalendars()
                tags = api.getAllTags()
            } catch (e: Exception) { statusMessage = e.message }
            isLoading = false
        }
    }

    fun refreshLists() {
        scope.launch {
            try {
                calendars = api.getCalendars()
                tags = api.getAllTags()
                defaultCalHref = api.getConfig().defaultCalendar
            } catch (e: Exception) { }
        }
    }

    LaunchedEffect(Unit) { fastStart() }

    NavHost(navController, startDestination = "home") {
        composable("home") {
            HomeScreen(
                api = api,
                calendars = calendars,
                tags = tags,
                defaultCalHref = defaultCalHref,
                isLoading = isLoading,
                onGlobalRefresh = { fastStart() },
                onSettings = { navController.navigate("settings") },
                onTaskClick = { uid -> navController.navigate("detail/$uid") },
                onDataChanged = { refreshLists() }
            )
        }
        composable("detail/{uid}") { backStackEntry ->
            val uid = backStackEntry.arguments?.getString("uid")
            if (uid != null) {
                TaskDetailScreen(
                    api = api,
                    uid = uid,
                    calendars = calendars,
                    onBack = { navController.popBackStack(); refreshLists() }
                )
            }
        }
        composable("settings") {
            SettingsScreen(api = api, onBack = { navController.popBackStack(); refreshLists() })
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun HomeScreen(
    api: CfaitMobile,
    calendars: List<MobileCalendar>,
    tags: List<MobileTag>,
    defaultCalHref: String?,
    isLoading: Boolean,
    onGlobalRefresh: () -> Unit,
    onSettings: () -> Unit,
    onTaskClick: (String) -> Unit,
    onDataChanged: () -> Unit
) {
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    var sidebarTab by remember { mutableIntStateOf(0) }
    
    var tasks by remember { mutableStateOf<List<MobileTask>>(emptyList()) }
    var searchQuery by remember { mutableStateOf("") }
    var filterTag by remember { mutableStateOf<String?>(null) }
    var isSearchActive by remember { mutableStateOf(false) }
    var newTaskText by remember { mutableStateOf("") }
    
    val clipboardManager = LocalClipboardManager.current
    val isDark = isSystemInDarkTheme()

    val calColorMap = remember(calendars) { 
        calendars.associate { it.href to (it.color?.let { hex -> parseHexColor(hex) } ?: Color.Gray) }
    }

    BackHandler(enabled = drawerState.isOpen) {
        scope.launch { drawerState.close() }
    }

    fun updateTaskList() {
        scope.launch { try { tasks = api.getViewTasks(filterTag, searchQuery) } catch (_: Exception) { } }
    }

    LaunchedEffect(searchQuery, filterTag, isLoading, calendars, tags) { updateTaskList() }

    fun toggleTask(uid: String) = scope.launch { try { api.toggleTask(uid); updateTaskList(); onDataChanged() } catch (_: Exception){} }
    fun addTask(txt: String) = scope.launch { try { api.addTaskSmart(txt); updateTaskList(); onDataChanged() } catch (_: Exception){} }
    
    fun onTaskAction(action: String, task: MobileTask) {
        scope.launch {
            try {
                when(action) {
                    "delete" -> api.deleteTask(task.uid)
                    "cancel" -> api.setStatusCancelled(task.uid)
                    "playpause" -> api.setStatusProcess(task.uid)
                    "prio_up" -> api.changePriority(task.uid, 1)
                    "prio_down" -> api.changePriority(task.uid, -1)
                    "yank" -> clipboardManager.setText(AnnotatedString(task.uid))
                }
                updateTaskList()
                onDataChanged()
            } catch(_: Exception) {}
        }
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet {
                Column(modifier = Modifier.fillMaxHeight().width(300.dp)) {
                    PrimaryTabRow(selectedTabIndex = sidebarTab) {
                        Tab(selected = sidebarTab==0, onClick = { sidebarTab=0 }, text = { Text("Calendars") }, icon = { NfIcon(NfIcons.CALENDAR) })
                        Tab(selected = sidebarTab==1, onClick = { sidebarTab=1 }, text = { Text("Tags") }, icon = { NfIcon(NfIcons.TAG) })
                    }
                    LazyColumn(
                        modifier = Modifier.weight(1f),
                        contentPadding = PaddingValues(bottom = 24.dp)
                    ) {
                        if (sidebarTab == 0) {
                            items(calendars.filter { !it.isDisabled }) { cal ->
                                val calColor = cal.color?.let { parseHexColor(it) } ?: Color.Gray
                                val isDefault = cal.href == defaultCalHref
                                val iconChar = if (isDefault) NfIcons.WRITE_TARGET else if (cal.isVisible) NfIcons.VISIBLE else NfIcons.HIDDEN
                                val iconColor = if (isDefault) MaterialTheme.colorScheme.primary else if (cal.isVisible) calColor else Color.Gray

                                Row(
                                    modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    IconButton(onClick = { api.setCalendarVisibility(cal.href, !cal.isVisible); onDataChanged(); updateTaskList() }) {
                                        NfIcon(iconChar, color = iconColor)
                                    }
                                    TextButton(
                                        onClick = {
                                            api.setDefaultCalendar(cal.href)
                                            onDataChanged()
                                        },
                                        modifier = Modifier.weight(1f),
                                        colors = ButtonDefaults.textButtonColors(contentColor = if (isDefault) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface)
                                    ) {
                                        Text(
                                            cal.name,
                                            fontWeight = if (isDefault) FontWeight.Bold else FontWeight.Normal,
                                            modifier = Modifier.fillMaxWidth(),
                                            textAlign = TextAlign.Start
                                        )
                                    }
                                    IconButton(onClick = {
                                        scope.launch {
                                            api.isolateCalendar(cal.href)
                                            onDataChanged()
                                            drawerState.close()
                                        }
                                    }) {
                                        NfIcon(NfIcons.ARROW_RIGHT, size = 18.sp)
                                    }
                                }
                            }
                        } else {
                            item {
                                CompactTagRow(
                                    name = "All Tasks",
                                    count = null,
                                    color = MaterialTheme.colorScheme.onSurface,
                                    isSelected = filterTag == null,
                                    onClick = { filterTag = null; scope.launch { drawerState.close() } }
                                )
                            }
                            items(tags) { tag ->
                                val isUncat = tag.isUncategorized
                                val displayName = if (isUncat) "Uncategorized" else "#${tag.name}"
                                val isSel = if (isUncat) filterTag == ":::uncategorized:::" else filterTag == tag.name
                                val color = if (isUncat) Color.Gray else getTagColor(tag.name)
                                
                                CompactTagRow(
                                    name = displayName,
                                    count = tag.count.toInt(),
                                    color = color,
                                    isSelected = isSel,
                                    onClick = { 
                                        filterTag = if (isUncat) ":::uncategorized:::" else tag.name
                                        scope.launch { drawerState.close() } 
                                    }
                                )
                            }
                        }

                        // Logo at the end of the scrollable list
                        item {
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .heightIn(min = 150.dp) // Ensure it takes good space
                                    .padding(vertical = 32.dp),
                                contentAlignment = Alignment.Center
                            ) {
                                Image(
                                    painter = painterResource(id = R.drawable.ic_launcher_foreground),
                                    contentDescription = "Cfait Logo",
                                    modifier = Modifier.size(120.dp),
                                    // Removed ColorFilter to keep it colorful
                                    contentScale = ContentScale.Fit
                                )
                            }
                        }
                    }
                }
            }
        }
    ) {
        Scaffold(
            topBar = {
                if (isSearchActive) {
                    TopAppBar(
                        title = { TextField(value = searchQuery, onValueChange = { searchQuery = it }, placeholder = { Text("Search...") }, singleLine = true, colors = TextFieldDefaults.colors(focusedContainerColor = Color.Transparent, unfocusedContainerColor = Color.Transparent, focusedIndicatorColor = Color.Transparent, unfocusedIndicatorColor = Color.Transparent), modifier = Modifier.fillMaxWidth()) },
                        navigationIcon = { IconButton(onClick = { isSearchActive = false; searchQuery = "" }) { NfIcon(NfIcons.BACK, 20.sp) } }
                    )
                } else {
                    val headerTitle: @Composable () -> Unit = {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Image(
                                painter = painterResource(id = R.drawable.ic_launcher_foreground),
                                contentDescription = null,
                                modifier = Modifier.size(28.dp)
                            )
                            Spacer(Modifier.width(8.dp))
                            
                            val activeCalName = calendars.find { it.href == defaultCalHref }?.name ?: "Local"
                            
                            Text(
                                text = activeCalName,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f, fill = false)
                            )
                            
                            if (tasks.isNotEmpty()) {
                                Spacer(Modifier.width(8.dp))
                                Text(
                                    text = "(${tasks.size})",
                                    fontSize = 13.sp,
                                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
                                )
                            }
                        }
                    }

                    TopAppBar(
                        title = headerTitle,
                        navigationIcon = { IconButton(onClick = { scope.launch { drawerState.open() } }) { NfIcon(NfIcons.MENU, 20.sp) } },
                        actions = {
                            IconButton(onClick = { isSearchActive = true }) { NfIcon(NfIcons.SEARCH, 18.sp) }
                            if (isLoading) CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp) else IconButton(onClick = onGlobalRefresh) { NfIcon(NfIcons.REFRESH, 18.sp) }
                            IconButton(onClick = onSettings) { NfIcon(NfIcons.SETTINGS, 20.sp) }
                        }
                    )
                }
            },
            bottomBar = {
                Surface(tonalElevation = 3.dp) {
                    Row(Modifier.padding(16.dp).navigationBarsPadding(), verticalAlignment = Alignment.CenterVertically) {
                        OutlinedTextField(
                            value = newTaskText,
                            onValueChange = { newTaskText = it },
                            placeholder = { Text("!1 @tomorrow Buy milk") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true,
                            keyboardOptions = KeyboardOptions.Default.copy(imeAction = ImeAction.Send),
                            keyboardActions = KeyboardActions(onSend = {
                                if (newTaskText.isNotBlank()) {
                                    addTask(newTaskText)
                                    newTaskText = ""
                                }
                            })
                        )
                    }
                }
            }
        ) { padding ->
            LazyColumn(Modifier.padding(padding).fillMaxSize(), contentPadding = PaddingValues(bottom = 80.dp)) {
                items(tasks, key = { it.uid }) { task ->
                    val calColor = calColorMap[task.calendarHref] ?: Color.Gray
                    TaskRow(task, calColor, isDark, { toggleTask(task.uid) }, { act -> onTaskAction(act, task) }, onTaskClick)
                }
            }
        }
    }
}

// --- COMPACT TAG ROW ---
@Composable
fun CompactTagRow(name: String, count: Int?, color: Color, isSelected: Boolean, onClick: () -> Unit) {
    val bg = if (isSelected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(36.dp) 
            .background(bg, RoundedCornerShape(4.dp))
            .clickable { onClick() }
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        NfIcon(NfIcons.TAG, size = 14.sp, color = color)
        Spacer(Modifier.width(12.dp))
        Text(name, fontSize = 14.sp, modifier = Modifier.weight(1f), color = MaterialTheme.colorScheme.onSurface)
        if (count != null) {
            Text("$count", fontSize = 12.sp, color = Color.Gray)
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TaskRow(task: MobileTask, calColor: Color, isDark: Boolean, onToggle: () -> Unit, onAction: (String) -> Unit, onClick: (String) -> Unit) {
    val startPadding = (task.depth.toInt() * 12).dp 
    var expanded by remember { mutableStateOf(false) }
    
    val textColor = getTaskTextColor(task.priority.toInt(), task.isDone, isDark)

    Card(
        modifier = Modifier.fillMaxWidth().padding(start = 12.dp + startPadding, end = 12.dp, top = 2.dp, bottom = 2.dp).clickable { onClick(task.uid) },
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
    ) {
        Row(Modifier.padding(horizontal = 8.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            
            TaskCheckbox(task, calColor, onToggle)
            
            Spacer(Modifier.width(8.dp))

            Column(Modifier.weight(1f)) {
                Text(
                    text = task.summary, 
                    style = MaterialTheme.typography.bodyMedium,
                    color = textColor,
                    fontWeight = if(task.priority > 0.toUByte()) FontWeight.Medium else FontWeight.Normal,
                    textDecoration = if (task.isDone) TextDecoration.LineThrough else null,
                    lineHeight = 18.sp
                )
                
                FlowRow(modifier = Modifier.padding(top = 2.dp), horizontalArrangement = Arrangement.spacedBy(4.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    if (task.isBlocked) NfIcon(NfIcons.BLOCK, 10.sp, MaterialTheme.colorScheme.error)
                    if (!task.dueDateIso.isNullOrEmpty()) { NfIcon(NfIcons.CALENDAR, 10.sp, Color.Gray); Text(task.dueDateIso!!.take(10), fontSize = 10.sp, color = Color.Gray) }
                    if (task.isRecurring) NfIcon(NfIcons.REPEAT, 10.sp, Color.Gray)
                    
                    task.categories.forEach { tag ->
                        Text("#$tag", fontSize = 10.sp, color = getTagColor(tag), modifier = Modifier.padding(end = 2.dp))
                    }
                }
            }
            
            Box {
                IconButton(onClick = { expanded = true }, modifier = Modifier.size(24.dp)) { NfIcon(NfIcons.DOTS_CIRCLE, 16.sp) }
                DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                    DropdownMenuItem(text = { Text("Edit") }, onClick = { expanded = false; onClick(task.uid) }, leadingIcon = { NfIcon(NfIcons.EDIT, 16.sp) })
                    DropdownMenuItem(text = { Text(if (task.statusString == "InProcess") "Pause" else "Start") }, onClick = { expanded = false; onAction("playpause") }, leadingIcon = { NfIcon(if (task.statusString == "InProcess") NfIcons.PAUSE else NfIcons.PLAY, 16.sp) })
                    DropdownMenuItem(text = { Text("Increase Prio") }, onClick = { expanded = false; onAction("prio_up") }, leadingIcon = { NfIcon(NfIcons.PRIORITY_UP, 16.sp) })
                    DropdownMenuItem(text = { Text("Decrease Prio") }, onClick = { expanded = false; onAction("prio_down") }, leadingIcon = { NfIcon(NfIcons.PRIORITY_DOWN, 16.sp) })
                    DropdownMenuItem(text = { Text("Yank (Copy ID)") }, onClick = { expanded = false; onAction("yank") }, leadingIcon = { NfIcon(NfIcons.COPY, 16.sp) })
                    if (task.statusString != "Cancelled") {
                        DropdownMenuItem(text = { Text("Cancel") }, onClick = { expanded = false; onAction("cancel") }, leadingIcon = { NfIcon(NfIcons.CROSS, 16.sp) })
                    }
                    DropdownMenuItem(text = { Text("Delete", color = MaterialTheme.colorScheme.error) }, onClick = { expanded = false; onAction("delete") }, leadingIcon = { NfIcon(NfIcons.DELETE, 16.sp, MaterialTheme.colorScheme.error) })
                }
            }
        }
    }
}

@Composable
fun TaskCheckbox(task: MobileTask, calColor: Color, onClick: () -> Unit) {
    val isDone = task.isDone
    val status = task.statusString

    val bgColor = when {
        isDone -> Color(0xFF009900)
        status == "InProcess" -> Color(0xFF99CC99)
        status == "Cancelled" -> Color(0xFF4D3333)
        else -> Color.Transparent
    }

    Box(
        modifier = Modifier
            .size(20.dp)
            .background(bgColor, RoundedCornerShape(4.dp))
            .border(1.5.dp, calColor, RoundedCornerShape(4.dp))
            .clickable { onClick() },
        contentAlignment = Alignment.Center
    ) {
        if (isDone) {
            NfIcon(NfIcons.CHECK, 12.sp, Color.White)
        } else if (status == "InProcess") {
            Box(Modifier.offset(y = (-2).dp)) {
                NfIcon(NfIcons.PLAY, 10.sp, Color.White)
            }
        } else if (status == "Cancelled") {
            NfIcon(NfIcons.CROSS, 12.sp, Color.White)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskDetailScreen(api: CfaitMobile, uid: String, calendars: List<MobileCalendar>, onBack: () -> Unit) {
    var task by remember { mutableStateOf<MobileTask?>(null) }
    val scope = rememberCoroutineScope()
    var smartInput by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var showMoveDialog by remember { mutableStateOf(false) }
    val context = LocalContext.current

    LaunchedEffect(uid) {
        val all = api.getViewTasks(null, "")
        task = all.find { it.uid == uid }
        task?.let { smartInput = it.smartString; description = it.description }
    }

    if (task == null) { Box(Modifier.fillMaxSize()) { CircularProgressIndicator(Modifier.align(Alignment.Center)) }; return }

    if (showMoveDialog) {
        AlertDialog(
            onDismissRequest = { showMoveDialog = false },
            title = { Text("Move to Calendar") },
            text = {
                LazyColumn {
                    items(calendars) { cal ->
                        if (cal.href != task!!.calendarHref) {
                            TextButton(onClick = { scope.launch { api.moveTask(uid, cal.href); showMoveDialog = false; onBack() } }, modifier = Modifier.fillMaxWidth()) { Text(cal.name) }
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { showMoveDialog = false }) { Text("Cancel") } }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Edit Task") },
                navigationIcon = { IconButton(onClick = onBack) { NfIcon(NfIcons.BACK, 20.sp) } },
                actions = {
                    TextButton(onClick = { showMoveDialog = true }) { Text("Move") }
                    TextButton(onClick = {
                        scope.launch {
                            try {
                                api.updateTaskSmart(uid, smartInput)
                                api.updateTaskDescription(uid, description)
                                onBack() // Navigate back on success
                            } catch (e: Exception) {
                                Toast.makeText(context, "Failed to save: ${e.message}", Toast.LENGTH_LONG).show()
                            }
                        }
                    }) { Text("Save") }
                }
            )
        }
    ) { p ->
        Column(modifier = Modifier.padding(p).padding(16.dp)) {
            OutlinedTextField(value = smartInput, onValueChange = { smartInput = it }, label = { Text("Task (Smart Syntax)") }, modifier = Modifier.fillMaxWidth())
            Text("Use !1, @date, #tag, ~duration", style = MaterialTheme.typography.bodySmall, color = Color.Gray, modifier = Modifier.padding(start = 4.dp, bottom = 16.dp))
            
            if (task!!.blockedByNames.isNotEmpty()) {
                Text("Blocked By:", color = MaterialTheme.colorScheme.error, fontWeight = FontWeight.Bold, fontSize = 14.sp)
                task!!.blockedByNames.forEach { name ->
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 2.dp)) {
                        NfIcon(NfIcons.BLOCK, 12.sp, MaterialTheme.colorScheme.error)
                        Spacer(Modifier.width(4.dp))
                        Text(name, fontSize = 14.sp)
                    }
                }
                HorizontalDivider(Modifier.padding(vertical = 8.dp))
            }

            OutlinedTextField(
                value = description, 
                onValueChange = { description = it }, 
                label = { Text("Description") }, 
                modifier = Modifier.fillMaxWidth().weight(1f), 
                textStyle = TextStyle(textAlign = androidx.compose.ui.text.style.TextAlign.Start)
            )
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
    var aliases by remember { mutableStateOf<Map<String, List<String>>>(emptyMap()) }
    var newAliasKey by remember { mutableStateOf("") }
    var newAliasTags by remember { mutableStateOf("") }
    var allCalendars by remember { mutableStateOf<List<MobileCalendar>>(emptyList()) }
    var disabledSet by remember { mutableStateOf<Set<String>>(emptySet()) }

    val scope = rememberCoroutineScope()

    fun reload() {
        val cfg = api.getConfig()
        url = cfg.url
        user = cfg.username
        insecure = cfg.allowInsecure
        hideCompleted = cfg.hideCompleted
        aliases = cfg.tagAliases
        allCalendars = api.getCalendars()
        disabledSet = allCalendars.filter { it.isDisabled }.map { it.href }.toSet()
    }

    LaunchedEffect(Unit) { reload() }

    fun save() {
        scope.launch { 
            status = "Connecting..."
            try { 
                api.saveConfig(url, user, pass, insecure, hideCompleted, disabledSet.toList())
                status = api.connect(url, user, pass, insecure) 
                reload()
            } catch (e: Exception) { status = "Error: ${e.message}" } 
        }
    }

    Scaffold(topBar = { TopAppBar(title = { Text("Settings") }, navigationIcon = { IconButton(onClick = onBack) { NfIcon(NfIcons.BACK, 20.sp) } }) }) { p ->
        LazyColumn(modifier = Modifier.padding(p).padding(16.dp)) {
            item {
                Text("Connection", fontWeight = FontWeight.Bold, modifier = Modifier.padding(vertical = 8.dp))
                OutlinedTextField(value = url, onValueChange = { url = it }, label = { Text("CalDAV URL") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = user, onValueChange = { user = it }, label = { Text("Username") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = pass, onValueChange = { pass = it }, label = { Text("Password") }, visualTransformation = PasswordVisualTransformation(), modifier = Modifier.fillMaxWidth())
                Row(verticalAlignment = Alignment.CenterVertically) { Checkbox(checked = insecure, onCheckedChange = { insecure = it }); Text("Allow Insecure SSL") }
                Row(verticalAlignment = Alignment.CenterVertically) { Checkbox(checked = hideCompleted, onCheckedChange = { hideCompleted = it }); Text("Hide Completed Tasks") }
                
                Button(onClick = { save() }, modifier = Modifier.fillMaxWidth()) { Text("Save & Connect") }
                Text(status, color = if (status.startsWith("Error")) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary)
                
                HorizontalDivider(Modifier.padding(vertical = 16.dp))
                Text("Manage Calendars", fontWeight = FontWeight.Bold)
            }
            items(allCalendars) { cal ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(
                        checked = !disabledSet.contains(cal.href),
                        onCheckedChange = { enabled ->
                            val newSet = disabledSet.toMutableSet()
                            if (enabled) newSet.remove(cal.href) else newSet.add(cal.href)
                            disabledSet = newSet
                        }
                    )
                    Text(cal.name)
                }
            }

            item {
                HorizontalDivider(Modifier.padding(vertical = 16.dp))
                Text("Tag Aliases", fontWeight = FontWeight.Bold)
            }
            items(aliases.keys.toList()) { key ->
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 4.dp)) {
                    Text("#$key", fontWeight = FontWeight.Bold, modifier = Modifier.width(80.dp))
                    Text("→", modifier = Modifier.padding(horizontal = 8.dp))
                    Text(aliases[key]?.joinToString(", ") ?: "", modifier = Modifier.weight(1f))
                    IconButton(onClick = { scope.launch { api.removeAlias(key); reload() } }) { NfIcon(NfIcons.CROSS, 16.sp, MaterialTheme.colorScheme.error) }
                }
            }
            item {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(top = 8.dp)) {
                    OutlinedTextField(value = newAliasKey, onValueChange = { newAliasKey = it }, label = { Text("Alias") }, modifier = Modifier.weight(1f))
                    Spacer(Modifier.width(8.dp))
                    OutlinedTextField(value = newAliasTags, onValueChange = { newAliasTags = it }, label = { Text("Tags (comma)") }, modifier = Modifier.weight(1f))
                    IconButton(onClick = { if (newAliasKey.isNotBlank() && newAliasTags.isNotBlank()) { val tags = newAliasTags.split(",").map { it.trim().trimStart('#') }.filter { it.isNotEmpty() }; scope.launch { api.addAlias(newAliasKey.trimStart('#'), tags); newAliasKey=""; newAliasTags=""; reload() } } }) { NfIcon(NfIcons.ADD) }
                }
            }
        }
    }
}

// --- UTILS ---

fun parseHexColor(hex: String): Color {
    return try {
        var clean = hex.removePrefix("#")
        if (clean.length > 6) { clean = clean.take(6) }
        val colorInt = android.graphics.Color.parseColor("#$clean")
        Color(colorInt)
    } catch (e: Exception) { Color.Gray }
}

fun getTaskTextColor(prio: Int, isDone: Boolean, isDark: Boolean): Color {
    if (isDone) return Color.Gray
    return when(prio) {
        1 -> Color(0xFFFF4444)
        2 -> Color(0xFFFF6633)
        3 -> Color(0xFFFF8800)
        4 -> Color(0xFFFFBB33)
        5 -> Color(0xFFFFD700)
        6 -> Color(0xFFD9D98C)
        7 -> Color(0xFFB3BFC6)
        8 -> Color(0xFFA699CC)
        9 -> Color(0xFF998CA6)
        else -> if (isDark) Color.White else Color.Black
    }
}

@Composable
fun NfIcon(text: String, size: androidx.compose.ui.unit.TextUnit = 24.sp, color: Color = MaterialTheme.colorScheme.onSurface) {
    Text(text = text, fontFamily = NerdFont, fontSize = size, color = color)
}

fun getTagColor(tag: String): Color {
    val hash = tag.hashCode()
    val h = (kotlin.math.abs(hash) % 360).toFloat()
    return Color.hsv(h, 0.6f, 0.5f)
}